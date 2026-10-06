//! `rule:packaging/a-value-crosses-as-its-wit-type`: every row of the table converts a Novis value to
//! its WIT value, crosses into a guest and comes back as the same value.
//!
//! The guest echoes its argument from one export per row, and it is loaded by the loader, so its
//! manifest is checked against every export's WIT types first. Every export returns
//! `result<T, error>` through memory at 16: its case at 16, then the `T` at 20 when `T` aligns to 4
//! or less, and at 24 when it aligns to 8. A string or a list is a pointer and a length. `realloc` is
//! a bump allocator from 1024, which is where the host writes a string or a list it passes in. The
//! guest's own types are exported by the component, the way a built extension names them.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;

use nvs_ext::call::{Failure, Host, Meter, Request};
use nvs_ext::convert::{Key, Value, from_wit, to_wit};
use nvs_ext::load::{Entry, Extension, Loader, Refused, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::pack::append_section;
use nvs_ext::section::MANIFEST;
use nvs_ext::types::{CORE_CLASSES, NovisType};
use wasmtime::component::Val;

const GUEST: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))
    (type $i (record (field "seconds" s64) (field "nanos" s64)))
    (export "instant" (type (eq $i)))))
  (alias export $types "error" (type $error))
  (alias export $types "instant" (type $instant))
  (type $item-def (record (field "item-name" string) (field "count" (option s64))))
  (export $item "item" (type $item-def))
  (type $unit-def (enum "metres" "nautical-miles"))
  (export $unit "unit" (type $unit-def))
  (type $circle-def (record (field "radius" f64)))
  (export $circle "circle" (type $circle-def))
  (type $box-def (record (field "width" f64) (field "height" f64)))
  (export $box "box" (type $box-def))
  (type $area-def (variant (case "circle" $circle) (case "box" $box)))
  (export $area "area" (type $area-def))
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
      (i32.const 16))
    (func (export "echo-variant") (param i32 f64 f64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store8 (i32.const 24) (local.get 0))
      (f64.store (i32.const 32) (local.get 1))
      (f64.store (i32.const 40) (local.get 2))
      (i32.const 16))
    (func (export "echo-pair") (param i64 i64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i64.store (i32.const 24) (local.get 0))
      (i64.store (i32.const 32) (local.get 1))
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
  (func $enum (param "value" $unit) (result (result $unit (error $error)))
    (canon lift (core func $i "echo-bool") (memory $mem) (realloc $realloc)))
  (func $union (param "value" $area) (result (result $area (error $error)))
    (canon lift (core func $i "echo-variant") (memory $mem) (realloc $realloc)))
  (func $core (param "value" $instant) (result (result $instant (error $error)))
    (canon lift (core func $i "echo-pair") (memory $mem) (realloc $realloc)))
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
    (export "echo-shape" (func $shape))
    (export "echo-unit" (func $enum))
    (export "echo-area" (func $union))
    (export "echo-instant" (func $core)))
  (export "shop:shapes/api" (instance $api)))"#;

/// Each export's method, and the Novis type of its one parameter and its return.
const ROWS: [(&str, &str); 13] = [
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
    ("echoUnit", "Unit"),
    ("echoArea", "Area"),
    ("echoInstant", "Core\\Time\\Instant"),
];

/// The enum the manifest declares.
const UNIT: &str = r#"{"name": "Unit", "cases": ["Metres", "NauticalMiles"]}"#;

/// The union of shapes the manifest declares.
const AREA: &str = r#"{"name": "Area", "cases": [{"name": "Circle", "shape": "{radius: float}"}, {"name": "Box", "shape": "{width: float, height: float}"}]}"#;

/// The manifest of the guest, with `enums` and `unions` as its declared types.
fn manifest(enums: &str, unions: &str) -> String {
    let methods: Vec<String> = ROWS
        .iter()
        .map(|(name, ty)| {
            let ty = ty.replace('\\', "\\\\");
            format!(
                r#"{{"name": "{name}", "params": [{{"name": "value", "type": "{ty}"}}], "returns": "{ty}"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Shapes", "interface": "shop:shapes/api", "enums": [{enums}], "unions": [{unions}], "methods": [{}]}}"#,
        methods.join(", ")
    )
}

/// A host linking the type instance the guest imports, which offers types and no function.
fn host() -> Host {
    Host::new(8, |linker| {
        linker.instance("nvs:ext/types@1.0.0")?;
        Ok(())
    })
    .expect("the host starts")
}

/// The guest under `manifest`, as the loader loads it or refuses it.
fn load(host: &Host, manifest: &str) -> Result<Extension, Refused> {
    let component = wat::parse_str(GUEST).expect("the test component compiles");
    let bytes = append_section(component, MANIFEST, manifest.as_bytes());
    let entry = Entry {
        path: PathBuf::from("shapes.nvsx"),
        sha256: pin(&bytes),
        memory: None,
    };
    Loader::new(host.engine()).load_bytes(&entry, &bytes)
}

/// A host and the guest under it, loaded with its own manifest.
fn fixture() -> (Host, Extension) {
    let host = host();
    let extension = load(&host, &manifest(UNIT, AREA)).expect("the guest loads");
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
    let ty = extension
        .manifest
        .novis_type(text)
        .expect("the type resolves");
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

fn case(name: &str) -> Value {
    Value::Case(name.to_owned())
}

fn instant(seconds: i64, nanos: i64) -> Value {
    Value::Core {
        class: "Core\\Time\\Instant".to_owned(),
        fields: vec![
            ("seconds".to_owned(), Value::Int(seconds)),
            ("nanos".to_owned(), Value::Int(nanos)),
        ],
    }
}

fn floats(fields: &[(&str, f64)]) -> Value {
    Value::Array(
        fields
            .iter()
            .map(|(name, x)| (k(name), Value::Float(*x)))
            .collect(),
    )
}

fn wit_floats(fields: &[(&str, f64)]) -> Val {
    Val::Record(
        fields
            .iter()
            .map(|(name, x)| ((*name).to_owned(), Val::Float64(*x)))
            .collect(),
    )
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
    add("echoUnit", case("Metres"));
    add("echoUnit", case("NauticalMiles"));
    add("echoArea", floats(&[("radius", 1.5)]));
    add("echoArea", floats(&[("width", 2.0), ("height", 3.0)]));
    add("echoInstant", instant(1_700_000_000, 5));
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
fn an_enum_case_crosses_in_kebab_case() {
    let (host, extension) = fixture();
    let unit = extension
        .manifest
        .novis_type("Unit")
        .expect("the enum resolves");
    let nautical = Val::Enum("nautical-miles".to_owned());
    assert_eq!(
        to_wit(&unit, case("NauticalMiles")).expect("the case converts"),
        nautical
    );
    assert_eq!(
        from_wit(&unit, nautical).expect("the case converts back"),
        case("NauticalMiles")
    );
    assert!(
        to_wit(&unit, case("Miles")).is_err(),
        "a case the enum does not declare does not convert"
    );
    assert!(
        from_wit(&unit, Val::Enum("miles".to_owned())).is_err(),
        "a case the guest returns that the enum does not declare does not convert"
    );
    let request = request(&host);
    let value = case("NauticalMiles");
    assert_eq!(
        echo(&request, &extension, "echoUnit", "Unit", &value),
        value
    );

    let other = r#"{"name": "Unit", "cases": ["Metres", "Miles"]}"#;
    let err = load(&host, &manifest(other, AREA)).expect_err("the cases differ from the export's");
    assert!(err.reason.contains("echoUnit"), "{err}");
    let twice = r#"{"name": "Unit", "cases": ["NauticalMiles", "Nautical_Miles"]}"#;
    let err = load(&host, &manifest(twice, AREA)).expect_err("two cases are one WIT case");
    assert!(err.reason.contains("kebab-case"), "{err}");
}

#[test]
fn a_closed_union_of_shapes_crosses_as_a_variant() {
    let (host, extension) = fixture();
    let area = extension
        .manifest
        .novis_type("Area")
        .expect("the union resolves");
    let rect = [("width", 2.0), ("height", 3.0)];
    let variant = Val::Variant("box".to_owned(), Some(Box::new(wit_floats(&rect))));
    assert_eq!(
        to_wit(&area, floats(&[("height", 3.0), ("width", 2.0)])).expect("the shape converts"),
        variant,
        "the case is the one whose fields the keys are, in any order"
    );
    assert_eq!(
        from_wit(&area, variant).expect("the variant converts back"),
        floats(&rect)
    );
    assert!(
        to_wit(&area, floats(&[("radius", 1.0), ("width", 2.0)])).is_err(),
        "an array that is a value of no case does not convert"
    );
    let request = request(&host);
    let circle = floats(&[("radius", 0.5)]);
    assert_eq!(
        echo(&request, &extension, "echoArea", "Area", &circle),
        circle
    );

    let swapped = r#"{"name": "Area", "cases": [{"name": "Box", "shape": "{width: float, height: float}"}, {"name": "Circle", "shape": "{radius: float}"}]}"#;
    let err = load(&host, &manifest(UNIT, swapped)).expect_err("the cases are out of order");
    assert!(err.reason.contains("echoArea"), "{err}");
    let overlapping = r#"{"name": "Area", "cases": [{"name": "Circle", "shape": "{radius: float}"}, {"name": "Box", "shape": "{radius: float, width?: float}"}]}"#;
    let err = Manifest::parse(manifest(UNIT, overlapping).as_bytes())
        .expect_err("an array with only `radius` is a value of both cases");
    assert!(err.to_string().contains("cannot be told apart"), "{err}");
}

#[test]
fn a_core_value_class_crosses_as_its_types_record() {
    for core in CORE_CLASSES {
        assert_eq!(
            NovisType::parse(core.class),
            Ok(NovisType::Core(core)),
            "`{}` is not read as its record",
            core.class
        );
    }
    let ty = ty("Core\\Time\\Instant");
    let record = Val::Record(vec![
        ("seconds".to_owned(), Val::S64(-1)),
        ("nanos".to_owned(), Val::S64(999_999_999)),
    ]);
    assert_eq!(
        to_wit(&ty, instant(-1, 999_999_999)).expect("the instant converts"),
        record
    );
    assert_eq!(
        from_wit(&ty, record).expect("the record converts back"),
        instant(-1, 999_999_999)
    );
    let uuid = Value::Core {
        class: "Core\\Uuid".to_owned(),
        fields: vec![
            ("high".to_owned(), Value::Uint(1)),
            ("low".to_owned(), Value::Uint(2)),
        ],
    };
    assert!(
        to_wit(&ty, uuid).is_err(),
        "an instance of another class does not convert"
    );
    let (host, extension) = fixture();
    let request = request(&host);
    let value = instant(0, 1);
    assert_eq!(
        echo(
            &request,
            &extension,
            "echoInstant",
            "Core\\Time\\Instant",
            &value
        ),
        value
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

/// A guest that reads a `mixed` argument through the `value` accessors. `total` sums the `int`
/// elements under the key `prices`, and returns `-1` for a value that is not an array and `-2` for
/// one with no `prices`. `kind-of` returns the argument's kind. `keep` stores the handle of its
/// argument's first element in a global, and `stale` reads that handle in a later call. Each
/// export drops the borrow it received before it returns, as the canonical ABI requires. A memory
/// module of its own comes first, so the accessors are lowered into a memory that exists.
const VALUES: &str = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))
    (type $k (enum "null" "bool" "int" "uint" "float" "string" "bytes" "array" "object"))
    (export "kind" (type $kind (eq $k)))
    (export "value" (type $value (sub resource)))
    (export "[method]value.kind" (func (param "self" (borrow $value)) (result $kind)))
    (export "[method]value.as-int" (func (param "self" (borrow $value)) (result (option s64))))
    (export "[method]value.length" (func (param "self" (borrow $value)) (result (option u64))))
    (export "[method]value.element"
      (func (param "self" (borrow $value)) (param "index" u64) (result (option (own $value)))))
    (export "[method]value.field"
      (func (param "self" (borrow $value)) (param "name" string) (result (option (own $value)))))))
  (alias export $types "error" (type $error))
  (alias export $types "kind" (type $kind))
  (alias export $types "value" (type $value))
  (core module $memory
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 1024))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) (local $at i32)
      (local.set $at
        (i32.and
          (i32.add (global.get $bump) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (global.set $bump (i32.add (local.get $at) (local.get 3)))
      (local.get $at)))
  (core instance $mi (instantiate $memory))
  (alias core export $mi "memory" (core memory $mem))
  (alias core export $mi "realloc" (core func $realloc))
  (core func $kind (canon lower (func $types "[method]value.kind")))
  (core func $as-int (canon lower (func $types "[method]value.as-int") (memory $mem)))
  (core func $length (canon lower (func $types "[method]value.length") (memory $mem)))
  (core func $element (canon lower (func $types "[method]value.element") (memory $mem)))
  (core func $field (canon lower (func $types "[method]value.field") (memory $mem)))
  (core func $drop (canon resource.drop $value))
  (core module $m
    (import "host" "memory" (memory 1))
    (import "host" "kind" (func $kind (param i32) (result i32)))
    (import "host" "as-int" (func $as-int (param i32 i32)))
    (import "host" "length" (func $length (param i32 i32)))
    (import "host" "element" (func $element (param i32 i64 i32)))
    (import "host" "field" (func $field (param i32 i32 i32 i32)))
    (import "host" "drop" (func $drop (param i32)))
    (global $kept (mut i32) (i32.const 0))
    (data (i32.const 512) "prices")
    (func $ok (param i64) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i64.store (i32.const 24) (local.get 0))
      (i32.const 16))
    (func (export "total") (param $v i32) (result i32) (local $r i32)
      (local.set $r (call $total (local.get $v)))
      (call $drop (local.get $v))
      (local.get $r))
    (func $total (param $v i32) (result i32)
      (local $p i32) (local $n i64) (local $i i64) (local $e i32) (local $sum i64)
      (if (i32.ne (call $kind (local.get $v)) (i32.const 7))
        (then (return (call $ok (i64.const -1)))))
      (call $field (local.get $v) (i32.const 512) (i32.const 6) (i32.const 64))
      (if (i32.eqz (i32.load8_u (i32.const 64)))
        (then (return (call $ok (i64.const -2)))))
      (local.set $p (i32.load (i32.const 68)))
      (call $length (local.get $p) (i32.const 64))
      (local.set $n (i64.load (i32.const 72)))
      (block $done
        (loop $next
          (br_if $done (i64.ge_u (local.get $i) (local.get $n)))
          (call $element (local.get $p) (local.get $i) (i32.const 64))
          (local.set $e (i32.load (i32.const 68)))
          (call $as-int (local.get $e) (i32.const 80))
          (if (i32.load8_u (i32.const 80))
            (then (local.set $sum (i64.add (local.get $sum) (i64.load (i32.const 88))))))
          (call $drop (local.get $e))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (br $next)))
      (call $drop (local.get $p))
      (call $ok (local.get $sum)))
    (func (export "kind-of") (param $v i32) (result i32)
      (i32.store8 (i32.const 16) (i32.const 0))
      (i32.store8 (i32.const 20) (call $kind (local.get $v)))
      (call $drop (local.get $v))
      (i32.const 16))
    (func (export "keep") (param $v i32) (result i32)
      (call $element (local.get $v) (i64.const 0) (i32.const 64))
      (global.set $kept (i32.load (i32.const 68)))
      (call $drop (local.get $v))
      (call $ok (i64.const 0)))
    (func (export "stale") (result i32)
      (call $as-int (global.get $kept) (i32.const 80))
      (call $ok (i64.load (i32.const 88)))))
  (core instance $i (instantiate $m
    (with "host" (instance
      (export "memory" (memory $mem))
      (export "kind" (func $kind))
      (export "as-int" (func $as-int))
      (export "length" (func $length))
      (export "element" (func $element))
      (export "field" (func $field))
      (export "drop" (func $drop))))))
  (func $total (param "value" (borrow $value)) (result (result s64 (error $error)))
    (canon lift (core func $i "total") (memory $mem) (realloc $realloc)))
  (func $kind-of (param "value" (borrow $value)) (result (result $kind (error $error)))
    (canon lift (core func $i "kind-of") (memory $mem) (realloc $realloc)))
  (func $keep (param "value" (borrow $value)) (result (result s64 (error $error)))
    (canon lift (core func $i "keep") (memory $mem) (realloc $realloc)))
  (func $stale (result (result s64 (error $error)))
    (canon lift (core func $i "stale") (memory $mem) (realloc $realloc)))
  (instance $api
    (export "total" (func $total))
    (export "kind-of" (func $kind-of))
    (export "keep" (func $keep))
    (export "stale" (func $stale)))
  (export "shop:values/api" (instance $api)))"#;

/// The manifest of the `VALUES` guest.
const VALUES_MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Values", "interface": "shop:values/api",
  "enums": [{"name": "Kind", "cases": ["Null", "Bool", "Int", "Uint", "Float", "String", "Bytes", "Array", "Object"]}],
  "methods": [
    {"name": "total", "params": [{"name": "value", "type": "mixed"}], "returns": "int"},
    {"name": "kindOf", "params": [{"name": "value", "type": "mixed"}], "returns": "Kind"},
    {"name": "keep", "params": [{"name": "value", "type": "mixed"}], "returns": "int"},
    {"name": "stale", "returns": "int"}]}"#;

#[test]
fn a_mixed_argument_crosses_as_a_value_handle_read_through_accessors() {
    let host = host();
    let component = wat::parse_str(VALUES).expect("the test component compiles");
    let bytes = append_section(component, MANIFEST, VALUES_MANIFEST.as_bytes());
    let entry = Entry {
        path: PathBuf::from("values.nvsx"),
        sha256: pin(&bytes),
        memory: None,
    };
    let extension = Loader::new(host.engine())
        .load_bytes(&entry, &bytes)
        .expect("the guest loads");
    let request = request(&host);
    let call =
        |method: &str, args: Vec<Value>| block_on(request.call_values(&extension, method, args));
    let order = Value::Array(vec![
        (k("name"), s("lamp")),
        (
            k("prices"),
            list(vec![Value::Int(3), s("free"), Value::Int(4)]),
        ),
    ]);
    assert_eq!(
        call("total", vec![order.clone()]),
        Ok(Some(Value::Int(7))),
        "the guest reads the array through `field`, `length`, `element` and `as-int`"
    );
    assert_eq!(call("total", vec![Value::Int(5)]), Ok(Some(Value::Int(-1))));
    assert_eq!(
        call("total", vec![list(vec![Value::Int(1)])]),
        Ok(Some(Value::Int(-2)))
    );

    let kinds = [
        (Value::Null, "Null"),
        (Value::Bool(true), "Bool"),
        (Value::Int(1), "Int"),
        (Value::Uint(1), "Uint"),
        (Value::Float(1.5), "Float"),
        (s("text"), "String"),
        (Value::Bytes(vec![1]), "Bytes"),
        (list(vec![]), "Array"),
        (instant(0, 0), "Object"),
        (Value::Object("Shop\\Cart".to_owned()), "Object"),
        (case("Metres"), "Object"),
    ];
    for (value, kind) in kinds {
        assert_eq!(
            call("kindOf", vec![value.clone()]),
            Ok(Some(case(kind))),
            "{value:?}"
        );
    }

    assert_eq!(
        call("keep", vec![list(vec![Value::Int(9)])]),
        Ok(Some(Value::Int(0)))
    );
    match call("stale", vec![]) {
        Err(Failure::Trap(crash)) => assert!(
            crash.reason.contains("not one this call received"),
            "{crash:?}"
        ),
        other => panic!("a handle kept past its call reads nothing, and got {other:?}"),
    }
    assert_eq!(
        call("total", vec![order]),
        Ok(Some(Value::Int(7))),
        "the call after the trap runs on a fresh instance"
    );
    assert!(
        matches!(
            call("total", vec![]),
            Err(Failure::Error(nvs_ext::call::Error::Invalid(_)))
        ),
        "a call with the wrong number of arguments is invalid"
    );
}
