//! The `nvs:ext@1.0.0` world under `wit/nvs-ext/`, read with `wit-parser`
//! and held against `rule:packaging/a-value-crosses-as-its-wit-type` and
//! `rule:packaging/a-guest-has-no-ambient-authority`.
//!
//! Two rosters meet here. A `Core` value class is a [`CoreClass`] with
//! `slots`; each one either has its record in `nvs:ext/types` ([`CROSSES`])
//! or is in [`REFUSED`] with the one sentence that says why it cannot cross.
//! A class added to the registry fails the walk until it is placed in one of
//! the two. The world's imports are the other roster, written once in
//! [`ALLOWED_IMPORTS`]: the resolved world, with every interface its imports
//! are written in pulled in, must import exactly those.
//!
//! The WIT text is parsed from the repository, never embedded, so the test
//! and the file an extension author builds against cannot drift.

use std::collections::BTreeSet;

use nvs_stdlib::registry::{CLASSES, CoreClass};
use wit_parser::{
    Function, FunctionKind, InterfaceId, PackageId, Resolve, Type, TypeDefKind, TypeId, WorldItem,
    WorldKey,
};

/// The `Core` value classes that cross, each with its record in
/// `nvs:ext/types`.
const CROSSES: &[(&str, &str)] = &[
    ("Core\\BigInt", "big-int"),
    ("Core\\Uuid", "uuid"),
    ("Core\\Uri", "uri"),
    ("Core\\Crypto\\PublicKey", "public-key"),
    ("Core\\Time\\Instant", "instant"),
    ("Core\\Time\\DateTime", "date-time"),
    ("Core\\Time\\Date", "date"),
    ("Core\\Time\\TimeOfDay", "time-of-day"),
    ("Core\\Time\\Duration", "duration"),
    ("Core\\Time\\Zone", "zone"),
];

const HANDLE: &str = "It is a handle to a host resource, which a guest has no authority to use.";
const READER: &str =
    "It reads host input lazily, and a guest receives the items it needs as a `list`.";
const REPORT: &str =
    "It reports what a host service did, and a guest takes the fields it needs as scalars.";
const REQUEST: &str = "It belongs to the request the host is serving, and a guest takes the fields it needs as scalars.";
const STATE: &str = "It is an incremental state the host advances, and a guest receives the whole input as `list<u8>`.";
const SECRET: &str = "It contains secret key material, and a secret never leaves the host.";
const TERMINAL: &str = "It styles the host's own terminal, which a guest does not write to.";
const REFLECT: &str = "It describes the running program, which a guest cannot see.";
const TREE: &str = "It is a recursive tree, and WIT has no recursive types.";

/// The `Core` value classes that do not cross, each with why.
const REFUSED: &[(&str, &str)] = &[
    (
        "Core\\Regex\\Match",
        "It points into the subject the host matched, and a guest takes the groups it needs as strings.",
    ),
    (
        "Core\\Regex\\Pattern",
        "It is compiled for the host's regex engine, and a guest takes the pattern's text as a `string`.",
    ),
    ("Core\\IO\\Lines", READER),
    ("Core\\IO\\Walk", READER),
    ("Core\\IO\\File", HANDLE),
    ("Core\\IO\\Metadata", REPORT),
    ("Core\\Process\\Result", REPORT),
    ("Core\\Process\\Handle", HANDLE),
    (
        "Core\\ObjectMap",
        "It is keyed by object identity, which a copy does not keep.",
    ),
    (
        "Core\\ObjectSet",
        "It is keyed by object identity, which a copy does not keep.",
    ),
    (
        "Core\\Heap",
        "It contains a comparator closure, and a closure cannot cross.",
    ),
    (
        "Core\\Random\\Seeded",
        "Its state is a generator the host advances, and a guest draws random bytes from `wasi:random`.",
    ),
    ("Core\\Hash\\Stream", STATE),
    ("Core\\Router\\Match", REQUEST),
    ("Core\\Csv\\Rows", READER),
    ("Core\\Test\\Response", REQUEST),
    ("Core\\Test\\SentRequest", REQUEST),
    (
        "Core\\Task\\Channel",
        "It is shared between the host's tasks, and a guest has no tasks.",
    ),
    ("Core\\Script\\Handle", HANDLE),
    ("Core\\Script\\ExitReport", REPORT),
    ("Core\\Cli\\Text", TERMINAL),
    ("Core\\Cli\\Color", TERMINAL),
    ("Core\\Cli\\Style", TERMINAL),
    ("Core\\Cli\\Live", TERMINAL),
    ("Core\\Cli\\Progress", TERMINAL),
    ("Core\\Request\\Mount", REQUEST),
    ("Core\\Request\\BodyStream", REQUEST),
    ("Core\\Request\\Files", REQUEST),
    ("Core\\Request\\Part", REQUEST),
    ("Core\\Request\\PartContent", REQUEST),
    ("Core\\Socket\\Message", REQUEST),
    ("Core\\Sse\\Message", REQUEST),
    ("Core\\Crypto\\KeyPair", SECRET),
    ("Core\\Jwt\\KeySet", SECRET),
    ("Core\\Jwe\\Key", SECRET),
    (
        "Core\\Html\\Markup",
        "It is HTML the host trusts as safe, and a guest that returned one would make untrusted text trusted.",
    ),
    ("Core\\Xml\\Node", TREE),
    ("Core\\Xml\\Reader", STATE),
    ("Core\\Xml\\Writer", STATE),
    ("Core\\Compress\\Compressor", STATE),
    ("Core\\Compress\\Decompressor", STATE),
    (
        "Core\\Http\\Target",
        "It carries addresses the host's outbound policy approved, and a guest that returned one would skip that check.",
    ),
    ("Core\\Http\\Identity", SECRET),
    ("Core\\Http\\Response", REPORT),
    ("Core\\Http\\TlsInfo", REPORT),
    ("Core\\Http\\Socket", HANDLE),
    ("Core\\Http\\Stream", HANDLE),
    ("Core\\Http\\Event", REPORT),
    ("Core\\Http\\Events", READER),
    ("Core\\Http\\Lines", READER),
    ("Core\\Http\\Chunks", READER),
    (
        "Core\\Http\\Part",
        "It names a file on the host by path, which a guest has no authority to read.",
    ),
    ("Core\\Net\\Stream", HANDLE),
    ("Core\\Net\\Listener", HANDLE),
    ("Core\\Net\\Datagram", HANDLE),
    ("Core\\Net\\Datagram\\Message", REPORT),
    ("Core\\Cache\\Store", HANDLE),
    ("Core\\Cache\\SecretEntry", SECRET),
    ("Core\\RateLimit\\Decision", REPORT),
    ("Core\\Reflect\\ClassInfo", REFLECT),
    ("Core\\Reflect\\MethodInfo", REFLECT),
    ("Core\\Reflect\\PropertyInfo", REFLECT),
    ("Core\\Reflect\\ParameterInfo", REFLECT),
    ("Core\\Reflect\\ConstantInfo", REFLECT),
    ("Core\\Reflect\\AttributeInfo", REFLECT),
    ("Core\\Reflect\\EnumInfo", REFLECT),
    ("Core\\Ast\\Node", TREE),
    ("Core\\Db\\Connection", HANDLE),
    ("Core\\Db\\Transaction", HANDLE),
    ("Core\\Db\\Rows", REPORT),
    ("Core\\Db\\Stream", HANDLE),
    ("Core\\Db\\Row", REPORT),
    ("Core\\Db\\Write", REPORT),
    ("Core\\Db\\Column", REPORT),
    (
        "Core\\Db\\InList",
        "It is a list the host binds into SQL it builds, and a guest builds no SQL.",
    ),
    ("Core\\Db\\Schema", REPORT),
    ("Core\\Db\\Plan", REPORT),
    ("Core\\Db\\Plan\\Step", REPORT),
    ("Core\\Queue\\Id", REPORT),
    ("Core\\Queue\\Stats", REPORT),
];

/// Every interface the `extension` world imports once resolved: its own
/// three, the WASI ones 0246 § 2 allows, and the `wasi:io` interfaces those
/// are written in.
const ALLOWED_IMPORTS: &[&str] = &[
    "nvs:ext/types@1.0.0",
    "nvs:ext/log@1.0.0",
    "nvs:ext/settings@1.0.0",
    "wasi:cli/environment@0.2.12",
    "wasi:cli/exit@0.2.12",
    "wasi:cli/stdin@0.2.12",
    "wasi:cli/stdout@0.2.12",
    "wasi:cli/stderr@0.2.12",
    "wasi:clocks/monotonic-clock@0.2.12",
    "wasi:clocks/wall-clock@0.2.12",
    "wasi:random/random@0.2.12",
    "wasi:random/insecure@0.2.12",
    "wasi:random/insecure-seed@0.2.12",
    "wasi:filesystem/types@0.2.12",
    "wasi:filesystem/preopens@0.2.12",
    "wasi:http/types@0.2.12",
    "wasi:http/outgoing-handler@0.2.12",
    "wasi:io/error@0.2.12",
    "wasi:io/poll@0.2.12",
    "wasi:io/streams@0.2.12",
];

/// Each case of `value`'s `kind`, with the methods that read a value of it.
/// `null` has nothing to read, so `kind` itself is its reader.
const VALUE_READERS: &[(&str, &[&str])] = &[
    ("null", &["kind"]),
    ("bool", &["as-bool"]),
    ("int", &["as-int"]),
    ("uint", &["as-uint"]),
    ("float", &["as-float"]),
    ("string", &["as-string"]),
    ("bytes", &["as-bytes"]),
    ("array", &["length", "key", "element", "field"]),
    ("object", &["class-name"]),
];

fn world() -> (Resolve, PackageId) {
    let mut resolve = Resolve::default();
    let dir = nvs_repo::path("wit/nvs-ext");
    let (package, _) = resolve
        .push_dir(&dir)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", dir.display()));
    (resolve, package)
}

fn interface(resolve: &Resolve, package: PackageId, name: &str) -> InterfaceId {
    *resolve.packages[package]
        .interfaces
        .get(name)
        .unwrap_or_else(|| panic!("`nvs:ext` has no interface `{name}`"))
}

fn named_type(resolve: &Resolve, iface: InterfaceId, name: &str) -> TypeId {
    *resolve.interfaces[iface]
        .types
        .get(name)
        .unwrap_or_else(|| panic!("`nvs:ext/types` has no type `{name}`"))
}

/// A type the Novis-to-WIT table can produce. `u8` appears only as the
/// element of `list<u8>`, which is `bytes`.
fn check_crossable(resolve: &Resolve, ty: Type, at: &str) -> Result<(), String> {
    match ty {
        Type::Bool | Type::S64 | Type::U64 | Type::F64 | Type::String => Ok(()),
        Type::Id(id) => match &resolve.types[id].kind {
            TypeDefKind::Record(record) => record.fields.iter().try_for_each(|field| {
                check_crossable(resolve, field.ty, &format!("{at}.{}", field.name))
            }),
            TypeDefKind::List(Type::U8) => Ok(()),
            TypeDefKind::List(inner) | TypeDefKind::Option(inner) | TypeDefKind::Type(inner) => {
                check_crossable(resolve, *inner, at)
            }
            TypeDefKind::Tuple(tuple) => tuple
                .types
                .iter()
                .try_for_each(|inner| check_crossable(resolve, *inner, at)),
            TypeDefKind::Variant(variant) => variant
                .cases
                .iter()
                .filter_map(|case| case.ty)
                .try_for_each(|inner| check_crossable(resolve, inner, at)),
            TypeDefKind::Enum(_) => Ok(()),
            other => Err(format!(
                "{at} is a {other:?}, which no Novis type crosses as"
            )),
        },
        other => Err(format!(
            "{at} is `{other:?}`, which no Novis type crosses as"
        )),
    }
}

fn value_classes() -> Vec<&'static CoreClass> {
    CLASSES
        .iter()
        .filter(|class| !class.slots.is_empty())
        .collect()
}

fn method<'a>(resolve: &'a Resolve, types: InterfaceId, name: &str) -> &'a Function {
    resolve.interfaces[types]
        .functions
        .get(&format!("[method]value.{name}"))
        .unwrap_or_else(|| panic!("the `value` resource has no method `{name}`"))
}

#[test]
fn the_ext_world_parses_as_package_nvs_ext_at_1_0_0() {
    let (resolve, package) = world();
    let found = &resolve.packages[package];
    assert_eq!(found.name.to_string(), "nvs:ext@1.0.0");
    let interfaces: Vec<&str> = found.interfaces.keys().map(String::as_str).collect();
    assert_eq!(
        interfaces,
        ["log", "settings", "types"],
        "the package's interfaces"
    );
    let worlds: Vec<&str> = found.worlds.keys().map(String::as_str).collect();
    assert_eq!(worlds, ["extension"], "the package's worlds");
}

#[test]
fn every_core_value_class_crosses_as_a_types_record_or_is_named_as_refused() {
    let (resolve, package) = world();
    let types = interface(&resolve, package, "types");
    let classes = value_classes();
    let mut problems = Vec::new();

    for class in &classes {
        let crosses = CROSSES.iter().any(|(name, _)| *name == class.name);
        let refused = REFUSED.iter().any(|(name, _)| *name == class.name);
        match (crosses, refused) {
            (false, false) => problems.push(format!(
                "`{}` is a value class with no record in `nvs:ext/types` and no entry in REFUSED",
                class.name
            )),
            (true, true) => {
                problems.push(format!("`{}` is in both CROSSES and REFUSED", class.name))
            }
            _ => {}
        }
    }
    for (name, _) in CROSSES.iter().chain(REFUSED) {
        if !classes.iter().any(|class| class.name == *name) {
            problems.push(format!(
                "`{name}` is listed here but is not a `Core` value class"
            ));
        }
    }
    for (name, reason) in REFUSED {
        let sentences = reason.matches(". ").count();
        if !reason.ends_with('.') || sentences != 0 {
            problems.push(format!("`{name}`'s reason is not one sentence: {reason}"));
        }
    }

    let mut records = BTreeSet::new();
    for (class, record) in CROSSES {
        let id = named_type(&resolve, types, record);
        if !matches!(resolve.types[id].kind, TypeDefKind::Record(_)) {
            problems.push(format!("`{record}`, `{class}`'s type, is not a record"));
        }
        if let Err(err) = check_crossable(&resolve, Type::Id(id), record) {
            problems.push(format!("`{class}`: {err}"));
        }
        records.insert(*record);
    }
    for (name, id) in &resolve.interfaces[types].types {
        let is_record = matches!(resolve.types[*id].kind, TypeDefKind::Record(_));
        if is_record && !records.contains(name.as_str()) {
            problems.push(format!(
                "the record `{name}` is the type of no `Core` class in CROSSES"
            ));
        }
    }

    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_extension_world_imports_exactly_the_allowed_interfaces() {
    let (resolve, package) = world();
    let world = &resolve.worlds[resolve.packages[package].worlds["extension"]];
    let mut imported = BTreeSet::new();
    for (key, item) in &world.imports {
        match (key, item) {
            (WorldKey::Interface(id), WorldItem::Interface { .. }) => {
                imported.insert(
                    resolve
                        .id_of(*id)
                        .expect("an imported interface has a package"),
                );
            }
            (key, item) => panic!("the world imports {key:?} as {item:?}, not a named interface"),
        }
    }
    let allowed: BTreeSet<String> = ALLOWED_IMPORTS.iter().map(ToString::to_string).collect();
    let extra: Vec<&String> = imported.difference(&allowed).collect();
    let missing: Vec<&String> = allowed.difference(&imported).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the world imports {extra:?} that are not allowed, and misses {missing:?}"
    );
    assert!(
        world.exports.is_empty(),
        "the shared world exports nothing: {:?}",
        world.exports
    );
}

#[test]
fn the_types_error_variant_has_the_invalid_parse_and_runtime_cases() {
    let (resolve, package) = world();
    let types = interface(&resolve, package, "types");
    let TypeDefKind::Variant(variant) = &resolve.types[named_type(&resolve, types, "error")].kind
    else {
        panic!("`error` is not a variant");
    };
    let cases: Vec<(&str, Option<Type>)> = variant
        .cases
        .iter()
        .map(|case| (case.name.as_str(), case.ty))
        .collect();
    assert_eq!(
        cases,
        [
            ("invalid", Some(Type::String)),
            ("parse", Some(Type::String)),
            ("runtime", Some(Type::String))
        ]
    );
}

#[test]
fn the_value_resource_reads_every_kind_a_mixed_value_can_hold() {
    let (resolve, package) = world();
    let types = interface(&resolve, package, "types");
    let value = named_type(&resolve, types, "value");
    assert!(
        matches!(resolve.types[value].kind, TypeDefKind::Resource),
        "`value` is a resource"
    );

    let TypeDefKind::Enum(kind) = &resolve.types[named_type(&resolve, types, "kind")].kind else {
        panic!("`kind` is not an enum");
    };
    let cases: Vec<&str> = kind.cases.iter().map(|case| case.name.as_str()).collect();
    let expected: Vec<&str> = VALUE_READERS.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(cases, expected, "`kind`'s cases");

    for (kind, readers) in VALUE_READERS {
        for reader in *readers {
            let found = method(&resolve, types, reader);
            assert_eq!(
                found.kind,
                FunctionKind::Method(value),
                "`{reader}`, which reads `{kind}`"
            );
            let result = found
                .result
                .unwrap_or_else(|| panic!("`{reader}` returns nothing"));
            let readable = match result {
                Type::Id(id) => match &resolve.types[id].kind {
                    TypeDefKind::Option(_) => true,
                    TypeDefKind::Enum(_) => *reader == "kind",
                    _ => false,
                },
                _ => false,
            };
            assert!(
                readable,
                "`{reader}` returns an `option`, so a wrong kind reads `none`"
            );
        }
    }
}

#[test]
fn the_settings_interface_reads_every_setting_type_a_manifest_may_declare() {
    let (resolve, package) = world();
    let settings = interface(&resolve, package, "settings");
    let setting = *resolve.interfaces[settings]
        .types
        .get("setting")
        .expect("a `setting` type");
    let TypeDefKind::Variant(variant) = &resolve.types[setting].kind else {
        panic!("`setting` is not a variant");
    };
    let mut cases = Vec::new();
    for case in &variant.cases {
        let payload = match case.ty {
            Some(Type::Id(id)) => match resolve.types[id].kind {
                TypeDefKind::List(Type::String) => "list<string>".to_owned(),
                ref other => format!("{other:?}"),
            },
            Some(ty) => format!("{ty:?}"),
            None => "nothing".to_owned(),
        };
        cases.push((case.name.clone(), payload));
    }
    let expected: Vec<(String, String)> = [
        ("bool", "Bool"),
        ("int", "S64"),
        ("uint", "U64"),
        ("float", "F64"),
        ("string", "String"),
        ("strings", "list<string>"),
    ]
    .iter()
    .map(|(name, ty)| ((*name).to_owned(), (*ty).to_owned()))
    .collect();
    assert_eq!(
        cases, expected,
        "`setting`'s cases, one per type 0246 § 9 allows"
    );

    let get = resolve.interfaces[settings]
        .functions
        .get("get")
        .expect("a `get` function");
    let params: Vec<(&str, Type)> = get
        .params
        .iter()
        .map(|param| (param.name.as_str(), param.ty))
        .collect();
    assert_eq!(params, [("key", Type::String)], "`get`'s parameters");
    let returns_option_setting = matches!(
        get.result.map(|ty| match ty {
            Type::Id(id) => resolve.types[id].kind.clone(),
            _ => TypeDefKind::Unknown,
        }),
        Some(TypeDefKind::Option(Type::Id(inner))) if inner == setting
    );
    assert!(returns_option_setting, "`get` returns `option<setting>`");
}
