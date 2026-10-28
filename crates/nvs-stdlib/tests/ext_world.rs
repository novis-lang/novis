//! The `nvs:ext@1.0.0` world under `wit/nvs-ext/` and the image component's
//! `nvs:image@1.0.0` in `wit/image.wit`, read with `wit-parser` and held
//! against `rule:packaging/a-value-crosses-as-its-wit-type`,
//! `rule:packaging/a-guest-has-no-ambient-authority` and
//! `rule:core-classes/image-pipeline`.
//!
//! Three rosters meet here. A `Core` value class is a [`CoreClass`] with
//! `slots`; each one either has its record in `nvs:ext/types` (the extension
//! host's [`CORE_CLASSES`], whose fields are held to the record's) or is in
//! [`REFUSED`] with the one sentence that says why it cannot cross.
//! A class added to the registry fails the walk until it is placed in one of
//! the two. The world's imports are the second roster, written once in
//! [`ALLOWED_IMPORTS`]: the resolved world, with every interface its imports
//! are written in pulled in, must import exactly those. The third is
//! [`IMAGE_MEMBERS`]: every member of `Novis\Image`'s builder with the place
//! in `codec` it runs, so a member with no place, or a place no member
//! reaches, fails. `wit/intl.wit`'s `nvs:intl@1.0.0` is held to
//! `rule:core-classes/intl-batch-shape`: [`ICU_EXPORTS`] in order, and every
//! one but [`ICU_SINGLE`] takes a list first and returns a list.
//! `wit/pdf.wit`'s `nvs:pdf@1.0.0` is held to [`PDF_EXPORTS`] in order, and
//! its `render` to the two passes of
//! `rule:core-classes/pdf-header-is-a-page-function`.
//! `wit/spreadsheet.wit`'s `nvs:spreadsheet@1.0.0` is resource-shaped: it is
//! held to [`SPREADSHEET_EXPORTS`] in order and to [`SPREADSHEET_RESOURCES`],
//! each opened as an `own` and passed back as a `borrow`
//! (`rule:core-classes/spreadsheet-roster`).
//!
//! The WIT text is parsed from the repository, never embedded, so the test
//! and the file an extension author builds against cannot drift.

use std::collections::BTreeSet;

use nvs_ext::types::{CORE_CLASSES, NovisType};
use nvs_stdlib::registry::{CLASSES, CoreClass};
use wit_parser::{
    Function, FunctionKind, Handle, InterfaceId, PackageId, Resolve, Type, TypeDefKind, TypeId,
    WorldId, WorldItem, WorldKey,
};

/// Whether the record field `wit` is the WIT type the Novis type `ty` of a
/// `Core` record's field crosses as.
fn field_crosses_as(resolve: &Resolve, ty: &NovisType, wit: Type) -> bool {
    match (ty, wit) {
        (NovisType::Bool, Type::Bool)
        | (NovisType::Int, Type::S64)
        | (NovisType::Uint, Type::U64)
        | (NovisType::Float, Type::F64)
        | (NovisType::String, Type::String) => true,
        (NovisType::Bytes, Type::Id(id)) => {
            matches!(resolve.types[id].kind, TypeDefKind::List(Type::U8))
        }
        _ => false,
    }
}

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
        "Core\\Test\\Outcome",
        "It reports a test run to that test's own hook, and a guest runs no tests.",
    ),
    (
        "Core\\Test\\Summary",
        "It reports a test class to that class's own hook, and a guest runs no tests.",
    ),
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
    ("Core\\Ldap\\Connection", HANDLE),
    ("Core\\Ldap\\Entries", HANDLE),
    ("Core\\Ldap\\Changes", HANDLE),
    ("Core\\Ldap\\Entry", REPORT),
    (
        "Core\\Ldap\\Filter",
        "It is the BER encoding the host sends to a directory, and a guest opens no directory.",
    ),
    (
        "Core\\Ldap\\Change",
        "It is a change the host sends to a directory, and a guest opens no directory.",
    ),
    (
        "Core\\Ldap\\Dn",
        "It names an entry for the host to send to a directory, and a guest opens no directory.",
    ),
    (
        "Core\\Ldap\\Sid",
        "It is an account's identifier that a directory sends, and a guest opens no directory.",
    ),
    (
        "Core\\Ldap\\Ad\\AccountFlags",
        "It is an account's flags that a directory sends, and a guest opens no directory.",
    ),
    (
        "Core\\Ldap\\Ad\\GroupType",
        "It is a group's flags that a directory sends, and a guest opens no directory.",
    ),
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
    (
        "Core\\Passkey\\Credential",
        "Its key is checked against its algorithm when the host builds one, and a guest takes its four parts as scalars.",
    ),
    (
        "Core\\WebPush\\Subscription",
        "Its endpoint and keys are checked when the host reads one, and a guest takes the browser's JSON as a string.",
    ),
    ("Core\\WebPush\\Sent", REPORT),
];

/// Every interface the `extension` world imports once resolved: its own
/// three, the WASI ones `rule:packaging/a-guest-has-no-ambient-authority`
/// allows, and the `wasi:io` interfaces those are written in.
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
    ("bytes", &["as-bytes", "size", "range"]),
    ("array", &["length", "key", "element", "field"]),
    ("object", &["class-name"]),
];

/// The image component's `codec` exports, a closed set of nine
/// (`rule:core-classes/image-pipeline`).
const CODEC_EXPORTS: &[&str] = &[
    "info",
    "run",
    "variants",
    "compare",
    "hash",
    "placeholder",
    "palette",
    "qr",
    "barcode",
];

/// Where a member of `Novis\Image` runs.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// An export of `codec`.
    Export(&'static str),
    /// A case of `step`, which a terminal sends inside a `plan`.
    Step(&'static str),
    /// A case of `source`, where a pipeline starts.
    Source(&'static str),
    /// A case of `output`, which says what `run` returns.
    Output(&'static str),
    /// Novis source in `Novis\Image` that needs no codec.
    Novis,
}

use Place::{Export, Novis, Output, Source, Step};

/// Every member of `Novis\Image`, with where it runs
/// (`rule:core-classes/image-pipeline`,
/// `rule:core-classes/image-one-entry-point-per-job`). `measureText` and `fromRaw` start a pipeline from a `source`
/// case and `raw` is an `output` of `run`.
const IMAGE_MEMBERS: &[(&str, &[Place])] = &[
    ("Image::open", &[Source("encoded")]),
    ("Image::create", &[Source("canvas")]),
    ("Image::info", &[Export("info")]),
    ("$img->resize", &[Step("resize")]),
    ("$img->crop", &[Step("crop")]),
    ("$img->trim", &[Step("trim")]),
    ("$img->rotate", &[Step("rotate")]),
    ("$img->flip", &[Step("flip")]),
    ("$img->composite", &[Step("composite")]),
    ("$img->flatten", &[Step("flatten")]),
    ("$img->sharpen", &[Step("sharpen")]),
    ("$img->blur", &[Step("blur")]),
    ("$img->grayscale", &[Step("grayscale")]),
    ("$img->brightness", &[Step("brightness")]),
    ("$img->contrast", &[Step("contrast")]),
    ("$img->gamma", &[Step("gamma")]),
    ("$img->tint", &[Step("tint")]),
    ("$img->text", &[Step("text")]),
    ("Font::fromBytes", &[Novis]),
    (
        "Image::measureText",
        &[Source("text"), Export("run"), Output("size")],
    ),
    ("$img->format", &[Step("format")]),
    ("$img->metadata", &[Step("metadata")]),
    ("$img->encode", &[Export("run"), Output("encoded")]),
    ("$img->variants", &[Export("variants")]),
    ("$img->raw", &[Export("run"), Output("raw")]),
    ("Image::fromRaw", &[Source("pixels")]),
    ("Image::compare", &[Export("compare")]),
    ("Image::hash", &[Export("hash")]),
    ("Image::hashDistance", &[Novis]),
    ("Image::placeholder", &[Export("placeholder")]),
    ("Image::palette", &[Export("palette")]),
    ("Color::rgba", &[Novis]),
    ("Color::hex", &[Novis]),
    ("QrCode::render", &[Export("qr")]),
    ("QrCode::svg", &[Export("qr")]),
    ("QrCode::giroCode", &[Novis]),
    ("Barcode::render", &[Export("barcode")]),
    ("Barcode::svg", &[Export("barcode")]),
];

fn world() -> (Resolve, PackageId) {
    let mut resolve = Resolve::default();
    let dir = nvs_repo::path("wit/nvs-ext");
    let (package, _) = resolve
        .push_dir(&dir)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", dir.display()));
    (resolve, package)
}

/// `wit/image.wit`, resolved against the `nvs:ext` world it includes. The
/// second package is the image one.
fn image() -> (Resolve, PackageId, PackageId) {
    let (mut resolve, ext) = world();
    let file = nvs_repo::path("wit/image.wit");
    let package = resolve
        .push_file(&file)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", file.display()));
    (resolve, ext, package)
}

fn codec(resolve: &Resolve, package: PackageId) -> InterfaceId {
    *resolve.packages[package]
        .interfaces
        .get("codec")
        .expect("`nvs:image` has an interface `codec`")
}

/// The case names of the `codec` variant or enum `name`, or the field names of the record `name`.
fn codec_cases(resolve: &Resolve, codec: InterfaceId, name: &str) -> Vec<String> {
    let id = *resolve.interfaces[codec]
        .types
        .get(name)
        .unwrap_or_else(|| panic!("`codec` has no type `{name}`"));
    match &resolve.types[id].kind {
        TypeDefKind::Variant(variant) => variant.cases.iter().map(|c| c.name.clone()).collect(),
        TypeDefKind::Enum(cases) => cases.cases.iter().map(|c| c.name.clone()).collect(),
        TypeDefKind::Record(record) => record.fields.iter().map(|f| f.name.clone()).collect(),
        other => panic!("`{name}` is a {other:?}, not a variant, an enum or a record"),
    }
}

/// A type with every `type x = y` alias followed to its definition.
fn dealias(resolve: &Resolve, mut id: TypeId) -> TypeId {
    while let TypeDefKind::Type(Type::Id(inner)) = resolve.types[id].kind {
        id = inner;
    }
    id
}

/// The `ok` and `err` types of an export that returns a `result`.
fn result_of(resolve: &Resolve, function: &Function) -> Option<(Option<Type>, Option<Type>)> {
    match function.result? {
        Type::Id(id) => match &resolve.types[id].kind {
            TypeDefKind::Result(result) => Some((result.ok, result.err)),
            _ => None,
        },
        _ => None,
    }
}

/// Every interface a world imports, by its full name.
fn imports(resolve: &Resolve, world: WorldId) -> BTreeSet<String> {
    let mut imported = BTreeSet::new();
    for (key, item) in &resolve.worlds[world].imports {
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
    imported
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
        let crosses = CORE_CLASSES.iter().any(|core| core.class == class.name);
        let refused = REFUSED.iter().any(|(name, _)| *name == class.name);
        match (crosses, refused) {
            (false, false) => problems.push(format!(
                "`{}` is a value class with no record in `nvs:ext/types` and no entry in REFUSED",
                class.name
            )),
            (true, true) => problems.push(format!(
                "`{}` is in both CORE_CLASSES and REFUSED",
                class.name
            )),
            _ => {}
        }
    }
    let crossing = CORE_CLASSES.iter().map(|core| core.class);
    for name in crossing.chain(REFUSED.iter().map(|(name, _)| *name)) {
        if !classes.iter().any(|class| class.name == name) {
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
    for core in CORE_CLASSES {
        let (class, record) = (core.class, core.record);
        let id = named_type(&resolve, types, record);
        match &resolve.types[id].kind {
            TypeDefKind::Record(wit) => {
                let same = wit.fields.len() == core.fields.len()
                    && wit
                        .fields
                        .iter()
                        .zip(core.fields)
                        .all(|(field, (name, ty))| {
                            field.name == *name && field_crosses_as(&resolve, ty, field.ty)
                        });
                if !same {
                    problems.push(format!(
                        "`{record}`'s fields are not the ones `{class}` crosses with"
                    ));
                }
            }
            _ => problems.push(format!("`{record}`, `{class}`'s type, is not a record")),
        }
        if let Err(err) = check_crossable(&resolve, Type::Id(id), record) {
            problems.push(format!("`{class}`: {err}"));
        }
        records.insert(record);
    }
    for (name, id) in &resolve.interfaces[types].types {
        let is_record = matches!(resolve.types[*id].kind, TypeDefKind::Record(_));
        if is_record && !records.contains(name.as_str()) {
            problems.push(format!(
                "the record `{name}` is the type of no `Core` class in CORE_CLASSES"
            ));
        }
    }

    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_extension_world_imports_exactly_the_allowed_interfaces() {
    let (resolve, package) = world();
    let world_id = resolve.packages[package].worlds["extension"];
    let world = &resolve.worlds[world_id];
    let imported = imports(&resolve, world_id);
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
        "`setting`'s cases, one per type a settings block allows"
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

#[test]
fn the_image_codec_interface_resolves_against_the_ext_world() {
    let (resolve, _, package) = image();
    let found = &resolve.packages[package];
    assert_eq!(found.name.to_string(), "nvs:image@1.0.0");
    let interfaces: Vec<&str> = found.interfaces.keys().map(String::as_str).collect();
    assert_eq!(interfaces, ["codec"], "the package's interfaces");
    let worlds: Vec<&str> = found.worlds.keys().map(String::as_str).collect();
    assert_eq!(worlds, ["image"], "the package's worlds");

    let world_id = found.worlds["image"];
    let allowed: BTreeSet<String> = ALLOWED_IMPORTS.iter().map(ToString::to_string).collect();
    // `codec` uses `nvs:ext/types`, which the `extension` world already imports.
    let imported = imports(&resolve, world_id);
    let extra: Vec<&String> = imported.difference(&allowed).collect();
    let missing: Vec<&String> = allowed.difference(&imported).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the `image` world imports {extra:?} that the `extension` world does not, and misses {missing:?}"
    );

    let exports: Vec<String> = resolve.worlds[world_id]
        .exports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), WorldItem::Interface { .. }) => resolve
                .id_of(*id)
                .expect("an exported interface has a package"),
            (key, item) => panic!("the world exports {key:?} as {item:?}, not a named interface"),
        })
        .collect();
    assert_eq!(
        exports,
        ["nvs:image/codec@1.0.0"],
        "the `image` world exports `codec` alone"
    );
}

#[test]
fn the_image_codec_exports_exactly_the_nine_entry_points() {
    let (resolve, _, package) = image();
    let codec = codec(&resolve, package);
    let functions = &resolve.interfaces[codec].functions;
    let names: Vec<&str> = functions.keys().map(String::as_str).collect();
    assert_eq!(names, CODEC_EXPORTS, "`codec`'s functions, in order");
    for (name, function) in functions {
        assert_eq!(
            function.kind,
            FunctionKind::Freestanding,
            "`{name}` is a plain function, not a resource method"
        );
    }
}

#[test]
fn every_image_codec_export_returns_a_result_with_the_ext_error() {
    let (resolve, ext, package) = image();
    let codec = codec(&resolve, package);
    let error = named_type(&resolve, interface(&resolve, ext, "types"), "error");
    let mut problems = Vec::new();
    for (name, function) in &resolve.interfaces[codec].functions {
        match result_of(&resolve, function) {
            Some((Some(_), Some(Type::Id(err)))) if dealias(&resolve, err) == error => {}
            Some((ok, err)) => problems.push(format!(
                "`{name}` returns `result<{ok:?}, {err:?}>`, not a value and `nvs:ext/types`' `error`"
            )),
            None => problems.push(format!("`{name}` does not return a `result`")),
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_image_codec_types_use_only_wit_types_the_value_table_gives() {
    let (resolve, _, package) = image();
    let codec = codec(&resolve, package);
    let mut problems = Vec::new();
    for (name, function) in &resolve.interfaces[codec].functions {
        for param in &function.params {
            if let Err(err) =
                check_crossable(&resolve, param.ty, &format!("{name}({})", param.name))
            {
                problems.push(err);
            }
        }
        if let Some((ok, err)) = result_of(&resolve, function) {
            for (side, ty) in [("ok", ok), ("err", err)] {
                if let Some(ty) = ty
                    && let Err(err) = check_crossable(&resolve, ty, &format!("{name} -> {side}"))
                {
                    problems.push(err);
                }
            }
        }
    }
    for (name, id) in &resolve.interfaces[codec].types {
        if let Err(err) = check_crossable(&resolve, Type::Id(*id), name) {
            problems.push(err);
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn every_image_builder_member_maps_to_an_export_a_plan_step_or_novis_source() {
    let (resolve, _, package) = image();
    let codec = codec(&resolve, package);
    let exports: Vec<String> = resolve.interfaces[codec]
        .functions
        .keys()
        .cloned()
        .collect();
    let rosters = [
        ("export", exports),
        ("step", codec_cases(&resolve, codec, "step")),
        ("source", codec_cases(&resolve, codec, "source")),
        ("output", codec_cases(&resolve, codec, "output")),
    ];
    let mut reached: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut problems = Vec::new();

    for (member, places) in IMAGE_MEMBERS {
        if places.is_empty() {
            problems.push(format!("`{member}` has no place"));
        }
        for place in *places {
            let (roster, name) = match place {
                Export(name) => ("export", *name),
                Step(name) => ("step", *name),
                Source(name) => ("source", *name),
                Output(name) => ("output", *name),
                Novis => continue,
            };
            let (_, names) = rosters
                .iter()
                .find(|(kind, _)| *kind == roster)
                .expect("every roster is listed");
            if names.iter().any(|found| found == name) {
                reached.insert((roster, name));
            } else {
                problems.push(format!(
                    "`{member}` runs in the {roster} `{name}`, which `codec` does not have"
                ));
            }
        }
    }
    for (roster, names) in &rosters {
        for name in names {
            if !reached.contains(&(*roster, name.as_str())) {
                problems.push(format!(
                    "the {roster} `{name}` is the place of no `Novis\\Image` member"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// The exports of the intl component's `icu`, one per static method of `Novis\Intl\Icu`
/// (`rule:core-classes/intl-roster`).
const ICU_EXPORTS: &[&str] = &[
    "collate-order",
    "sort-keys",
    "format-numbers",
    "plural-categories",
    "format-date-times",
    "format-dates",
    "format-times",
    "format-relative",
    "format-lists",
    "word-segments",
    "sentence-segments",
    "resolve-locales",
    "negotiate",
];

/// The one `icu` export that is not a batch: its input is one request's
/// `Accept-Language` header (`rule:core-classes/intl-batch-shape`).
const ICU_SINGLE: &str = "negotiate";

/// `wit/intl.wit`, resolved against the `nvs:ext` world it includes: the
/// resolve, the `nvs:ext` package, the `nvs:intl` package and its `icu`.
fn intl() -> (Resolve, PackageId, PackageId, InterfaceId) {
    let (mut resolve, ext) = world();
    let file = nvs_repo::path("wit/intl.wit");
    let package = resolve
        .push_file(&file)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", file.display()));
    assert_eq!(resolve.packages[package].name.to_string(), "nvs:intl@1.0.0");
    let icu = *resolve.packages[package]
        .interfaces
        .get("icu")
        .expect("`nvs:intl` has an interface `icu`");
    (resolve, ext, package, icu)
}

/// Whether `ty`, with aliases followed, is a `list`.
fn is_list(resolve: &Resolve, ty: Type) -> bool {
    match ty {
        Type::Id(id) => matches!(
            resolve.types[dealias(resolve, id)].kind,
            TypeDefKind::List(_)
        ),
        _ => false,
    }
}

#[test]
fn the_intl_interface_parses_against_the_world_in_wit_types_alone() {
    let (resolve, ext, package, icu) = intl();
    let found = &resolve.packages[package];
    let interfaces: Vec<&str> = found.interfaces.keys().map(String::as_str).collect();
    assert_eq!(interfaces, ["icu"], "the package's interfaces");
    let worlds: Vec<&str> = found.worlds.keys().map(String::as_str).collect();
    assert_eq!(worlds, ["intl"], "the package's worlds");

    let world_id = found.worlds["intl"];
    let allowed: BTreeSet<String> = ALLOWED_IMPORTS.iter().map(ToString::to_string).collect();
    let imported = imports(&resolve, world_id);
    let extra: Vec<&String> = imported.difference(&allowed).collect();
    let missing: Vec<&String> = allowed.difference(&imported).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the `intl` world imports {extra:?} that the `extension` world does not, and misses {missing:?}"
    );
    let exports: Vec<String> = resolve.worlds[world_id]
        .exports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), WorldItem::Interface { .. }) => resolve
                .id_of(*id)
                .expect("an exported interface has a package"),
            (key, item) => panic!("the world exports {key:?} as {item:?}, not a named interface"),
        })
        .collect();
    assert_eq!(
        exports,
        ["nvs:intl/icu@1.0.0"],
        "the `intl` world exports `icu` alone"
    );

    let names: Vec<&str> = resolve.interfaces[icu]
        .functions
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ICU_EXPORTS, "`icu`'s functions, in order");

    let problems = crossing_problems(&resolve, ext, icu);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// Every way `iface` fails `rule:packaging/a-value-crosses-as-its-wit-type`:
/// a function that is not freestanding, a parameter, result or type no Novis
/// type crosses as, and a result whose error is not the world's `error`.
fn crossing_problems(resolve: &Resolve, ext: PackageId, iface: InterfaceId) -> Vec<String> {
    let error = named_type(resolve, interface(resolve, ext, "types"), "error");
    let mut problems = Vec::new();
    for (name, function) in &resolve.interfaces[iface].functions {
        if function.kind != FunctionKind::Freestanding {
            problems.push(format!("`{name}` is not a plain function"));
        }
        for param in &function.params {
            if let Err(err) = check_crossable(resolve, param.ty, &format!("{name}({})", param.name))
            {
                problems.push(err);
            }
        }
        match result_of(resolve, function) {
            Some((Some(ok), Some(Type::Id(err)))) if dealias(resolve, err) == error => {
                if let Err(err) = check_crossable(resolve, ok, &format!("{name} -> ok")) {
                    problems.push(err);
                }
            }
            Some((ok, err)) => problems.push(format!(
                "`{name}` returns `result<{ok:?}, {err:?}>`, not a value and `nvs:ext/types`' `error`"
            )),
            None => problems.push(format!("`{name}` does not return a `result`")),
        }
    }
    for (name, id) in &resolve.interfaces[iface].types {
        if let Err(err) = check_crossable(resolve, Type::Id(*id), name) {
            problems.push(err);
        }
    }
    problems
}

#[test]
fn every_intl_formatting_export_takes_a_list_and_returns_a_list() {
    let (resolve, _, _, icu) = intl();
    let mut problems = Vec::new();
    for (name, function) in &resolve.interfaces[icu].functions {
        let takes_list = function
            .params
            .first()
            .is_some_and(|param| is_list(&resolve, param.ty));
        let returns_list = matches!(
            result_of(&resolve, function),
            Some((Some(ok), _)) if is_list(&resolve, ok)
        );
        if name == ICU_SINGLE {
            assert!(
                !takes_list && !returns_list,
                "`{name}` is the named exception, so it takes one header and returns one tag"
            );
            continue;
        }
        if !takes_list {
            problems.push(format!(
                "`{name}` takes a single value first where a list would do"
            ));
        }
        if !returns_list {
            problems.push(format!("`{name}` does not return a list"));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// The exports of the PDF component's `engine`, one per static method of `Novis\Pdf\Engine`
/// (`rule:core-classes/pdf-roster`).
const PDF_EXPORTS: &[&str] = &["render", "merge", "read", "read-text"];

/// `wit/pdf.wit`, resolved against the `nvs:ext` world it includes: the
/// resolve, the `nvs:ext` package, the `nvs:pdf` package and its `engine`.
fn pdf() -> (Resolve, PackageId, PackageId, InterfaceId) {
    let (mut resolve, ext) = world();
    let file = nvs_repo::path("wit/pdf.wit");
    let package = resolve
        .push_file(&file)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", file.display()));
    assert_eq!(resolve.packages[package].name.to_string(), "nvs:pdf@1.0.0");
    let engine = *resolve.packages[package]
        .interfaces
        .get("engine")
        .expect("`nvs:pdf` has an interface `engine`");
    (resolve, ext, package, engine)
}

/// The fields of the record `ty` names, with aliases followed, by name.
fn record_fields(resolve: &Resolve, ty: Type) -> Vec<(String, Type)> {
    let Type::Id(id) = ty else {
        panic!("{ty:?} is not a record");
    };
    match &resolve.types[dealias(resolve, id)].kind {
        TypeDefKind::Record(record) => record
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.ty))
            .collect(),
        other => panic!("{ty:?} is a {other:?}, not a record"),
    }
}

/// The element type of the `list` `ty` names, with aliases followed.
fn list_element(resolve: &Resolve, ty: Type) -> Type {
    match ty {
        Type::Id(id) => match &resolve.types[dealias(resolve, id)].kind {
            TypeDefKind::List(inner) => *inner,
            other => panic!("{ty:?} is a {other:?}, not a list"),
        },
        other => panic!("{other:?} is not a list"),
    }
}

#[test]
fn the_pdf_interface_parses_against_the_world_in_wit_types_alone() {
    let (resolve, ext, package, engine) = pdf();
    let found = &resolve.packages[package];
    let interfaces: Vec<&str> = found.interfaces.keys().map(String::as_str).collect();
    assert_eq!(interfaces, ["engine"], "the package's interfaces");
    let worlds: Vec<&str> = found.worlds.keys().map(String::as_str).collect();
    assert_eq!(worlds, ["pdf"], "the package's worlds");

    let world_id = found.worlds["pdf"];
    let allowed: BTreeSet<String> = ALLOWED_IMPORTS.iter().map(ToString::to_string).collect();
    let imported = imports(&resolve, world_id);
    let extra: Vec<&String> = imported.difference(&allowed).collect();
    let missing: Vec<&String> = allowed.difference(&imported).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the `pdf` world imports {extra:?} that the `extension` world does not, and misses {missing:?}"
    );
    let exports: Vec<String> = resolve.worlds[world_id]
        .exports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), WorldItem::Interface { .. }) => resolve
                .id_of(*id)
                .expect("an exported interface has a package"),
            (key, item) => panic!("the world exports {key:?} as {item:?}, not a named interface"),
        })
        .collect();
    assert_eq!(
        exports,
        ["nvs:pdf/engine@1.0.0"],
        "the `pdf` world exports `engine` alone"
    );

    let names: Vec<&str> = resolve.interfaces[engine]
        .functions
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, PDF_EXPORTS, "`engine`'s functions, in order");

    let problems = crossing_problems(&resolve, ext, engine);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// `render` takes one `pass`: `whole` and `first` carry the document, and
/// `second` carries every page's header, footer and slot markup and nothing
/// of the document, which the instance kept from `first`. The result carries
/// each page's facts for the functions to read.
#[test]
fn the_pdf_render_export_carries_a_second_pass_for_page_markup() {
    let (resolve, _, _, engine) = pdf();
    let render = &resolve.interfaces[engine].functions["render"];
    let [param] = render.params.as_slice() else {
        panic!(
            "`render` takes {} parameters, not one `pass`",
            render.params.len()
        );
    };
    let Type::Id(pass) = param.ty else {
        panic!("`render`'s parameter is {:?}, not a variant", param.ty);
    };
    let TypeDefKind::Variant(variant) = &resolve.types[dealias(&resolve, pass)].kind else {
        panic!("`render`'s parameter is not a variant");
    };
    let cases: Vec<(&str, Option<Type>)> = variant
        .cases
        .iter()
        .map(|case| (case.name.as_str(), case.ty))
        .collect();
    let names: Vec<&str> = cases.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["whole", "first", "second"], "the passes, in order");

    let payload = |name: &str| {
        let (_, ty) = cases.iter().find(|(case, _)| *case == name).unwrap();
        ty.unwrap_or_else(|| panic!("the `{name}` pass carries nothing"))
    };
    let field_names = |ty: Type| -> Vec<String> {
        record_fields(&resolve, ty)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    };
    assert_eq!(field_names(payload("whole")), ["html", "assets", "options"]);
    let first = record_fields(&resolve, payload("first"));
    let names: Vec<&str> = first.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        ["document", "header-height", "footer-height"],
        "the `first` pass carries the document and the margins its functions fill, under keys a \
         Novis value tells apart from `whole`'s"
    );
    assert_eq!(
        first[0].1,
        payload("whole"),
        "both first passes carry the document"
    );

    let second = record_fields(&resolve, payload("second"));
    let [(name, pages)] = second.as_slice() else {
        panic!("the `second` pass carries {second:?}, not the pages' markup alone");
    };
    assert_eq!(name, "pages");
    assert_eq!(
        field_names(list_element(&resolve, *pages)),
        ["header", "footer", "slots"],
        "each page's markup"
    );

    let Some((Some(rendered), _)) = result_of(&resolve, render) else {
        panic!("`render` returns no value");
    };
    let rendered = record_fields(&resolve, rendered);
    let (_, pages) = rendered
        .iter()
        .find(|(name, _)| name == "pages")
        .expect("`rendered` carries the pages' facts");
    assert_eq!(
        field_names(list_element(&resolve, *pages)),
        ["number", "name", "strings", "marks"],
        "each page's facts"
    );
}

/// The exports of the spreadsheet component's `engine`, one per static method of
/// `Novis\Spreadsheet\Engine` (`rule:core-classes/spreadsheet-roster`).
const SPREADSHEET_EXPORTS: &[&str] = &[
    "writer-open",
    "writer-style",
    "writer-sheet",
    "writer-rows",
    "writer-part",
    "writer-define",
    "writer-cell",
    "writer-column",
    "writer-finish",
    "reader-open",
    "reader-sheets",
    "reader-header",
    "reader-rows",
    "reader-merged",
    "reader-links",
    "reader-notes",
    "reader-names",
    "reader-tables",
    "reader-images",
    "reader-properties",
    "reader-macros",
    "template-open",
    "template-set",
    "template-fill",
    "template-items",
    "template-finish",
];

/// The resources `engine` exports, each kept by the request that opened it.
const SPREADSHEET_RESOURCES: &[&str] = &["writer", "reading", "filling"];

/// The word each resource's exports start with. `reading`'s are `reader-` and `filling`'s are
/// `template-`, because a manifest's resource is a class in `Novis\Spreadsheet`, where `Reader`
/// and `Template` are the Novis classes over them.
fn calls_of(resource: &str) -> &str {
    match resource {
        "reading" => "reader",
        "filling" => "template",
        _ => resource,
    }
}

/// `wit/spreadsheet.wit`, resolved against the `nvs:ext` world it includes:
/// the resolve, the `nvs:ext` package, the `nvs:spreadsheet` package and its
/// `engine`.
fn spreadsheet() -> (Resolve, PackageId, PackageId, InterfaceId) {
    let (mut resolve, ext) = world();
    let file = nvs_repo::path("wit/spreadsheet.wit");
    let package = resolve
        .push_file(&file)
        .unwrap_or_else(|err| panic!("{} does not parse: {err:?}", file.display()));
    assert_eq!(
        resolve.packages[package].name.to_string(),
        "nvs:spreadsheet@1.0.0"
    );
    let engine = *resolve.packages[package]
        .interfaces
        .get("engine")
        .expect("`nvs:spreadsheet` has an interface `engine`");
    (resolve, ext, package, engine)
}

/// The resource a handle type points at, and whether it is a `borrow`.
fn handle_of(resolve: &Resolve, ty: Type) -> Option<(TypeId, bool)> {
    let Type::Id(id) = ty else {
        return None;
    };
    match resolve.types[dealias(resolve, id)].kind {
        TypeDefKind::Handle(Handle::Borrow(resource)) => Some((dealias(resolve, resource), true)),
        TypeDefKind::Handle(Handle::Own(resource)) => Some((dealias(resolve, resource), false)),
        _ => None,
    }
}

/// Every way a resource-shaped `iface` fails `rule:packaging/a-value-crosses-as-its-wit-type`:
/// [`crossing_problems`]' checks, with a resource crossing as a `borrow` in a parameter and an
/// `own` in a result, and an `ok` side that may be empty, as a `void` method's is.
fn resource_crossing_problems(
    resolve: &Resolve,
    ext: PackageId,
    iface: InterfaceId,
) -> Vec<String> {
    let error = named_type(resolve, interface(resolve, ext, "types"), "error");
    let mut problems = Vec::new();
    for (name, function) in &resolve.interfaces[iface].functions {
        if function.kind != FunctionKind::Freestanding {
            problems.push(format!("`{name}` is not a plain function"));
        }
        for param in &function.params {
            let at = format!("{name}({})", param.name);
            match handle_of(resolve, param.ty) {
                Some((_, true)) => {}
                Some((_, false)) => problems.push(format!("{at} takes an `own`, not a `borrow`")),
                None => {
                    if let Err(err) = check_crossable(resolve, param.ty, &at) {
                        problems.push(err);
                    }
                }
            }
        }
        match result_of(resolve, function) {
            Some((ok, Some(Type::Id(err)))) if dealias(resolve, err) == error => match ok
                .map(|ok| (ok, handle_of(resolve, ok)))
            {
                None | Some((_, Some((_, false)))) => {}
                Some((_, Some((_, true)))) => {
                    problems.push(format!("`{name}` returns a `borrow`, not an `own`"));
                }
                Some((ok, None)) => {
                    if let Err(err) = check_crossable(resolve, ok, &format!("{name} -> ok")) {
                        problems.push(err);
                    }
                }
            },
            Some((ok, err)) => problems.push(format!(
                "`{name}` returns `result<{ok:?}, {err:?}>`, not a value and `nvs:ext/types`' `error`"
            )),
            None => problems.push(format!("`{name}` does not return a `result`")),
        }
    }
    for (name, id) in &resolve.interfaces[iface].types {
        if matches!(
            resolve.types[dealias(resolve, *id)].kind,
            TypeDefKind::Resource
        ) {
            continue;
        }
        if let Err(err) = check_crossable(resolve, Type::Id(*id), name) {
            problems.push(err);
        }
    }
    problems
}

#[test]
fn the_spreadsheet_interface_parses_against_the_world_in_wit_types_alone() {
    let (resolve, ext, package, engine) = spreadsheet();
    let found = &resolve.packages[package];
    let interfaces: Vec<&str> = found.interfaces.keys().map(String::as_str).collect();
    assert_eq!(interfaces, ["engine"], "the package's interfaces");
    let worlds: Vec<&str> = found.worlds.keys().map(String::as_str).collect();
    assert_eq!(worlds, ["spreadsheet"], "the package's worlds");

    let world_id = found.worlds["spreadsheet"];
    let allowed: BTreeSet<String> = ALLOWED_IMPORTS.iter().map(ToString::to_string).collect();
    let imported = imports(&resolve, world_id);
    let extra: Vec<&String> = imported.difference(&allowed).collect();
    let missing: Vec<&String> = allowed.difference(&imported).collect();
    assert!(
        extra.is_empty() && missing.is_empty(),
        "the `spreadsheet` world imports {extra:?} that the `extension` world does not, and misses {missing:?}"
    );
    let exports: Vec<String> = resolve.worlds[world_id]
        .exports
        .iter()
        .map(|(key, item)| match (key, item) {
            (WorldKey::Interface(id), WorldItem::Interface { .. }) => resolve
                .id_of(*id)
                .expect("an exported interface has a package"),
            (key, item) => panic!("the world exports {key:?} as {item:?}, not a named interface"),
        })
        .collect();
    assert_eq!(
        exports,
        ["nvs:spreadsheet/engine@1.0.0"],
        "the `spreadsheet` world exports `engine` alone"
    );

    let names: Vec<&str> = resolve.interfaces[engine]
        .functions
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, SPREADSHEET_EXPORTS, "`engine`'s functions, in order");

    let problems = resource_crossing_problems(&resolve, ext, engine);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// `engine` exports a `writer`, a `reading` and a `filling`. Each one's `-open` export returns
/// an `own` of it, which the request keeps, and every other export named for it takes a
/// `borrow` of it first, so no call gives one away.
#[test]
fn the_spreadsheet_interface_exports_writer_reader_and_template_resources() {
    let (resolve, _, _, engine) = spreadsheet();
    let iface = &resolve.interfaces[engine];
    let resources: Vec<&str> = iface
        .types
        .iter()
        .filter(|(_, id)| matches!(resolve.types[**id].kind, TypeDefKind::Resource))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(resources, SPREADSHEET_RESOURCES, "`engine`'s resources");

    let mut problems = Vec::new();
    for resource in SPREADSHEET_RESOURCES {
        let id = iface.types[*resource];
        let prefix = format!("{}-", calls_of(resource));
        let open = format!("{}-open", calls_of(resource));
        let mut calls = 0;
        for (name, function) in iface
            .functions
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
        {
            calls += 1;
            if *name == open {
                let returned = result_of(&resolve, function)
                    .and_then(|(ok, _)| ok)
                    .and_then(|ok| handle_of(&resolve, ok));
                if returned != Some((id, false)) {
                    problems.push(format!("`{name}` does not return an `own<{resource}>`"));
                }
                continue;
            }
            let first = function
                .params
                .first()
                .and_then(|param| handle_of(&resolve, param.ty));
            if first != Some((id, true)) {
                problems.push(format!(
                    "`{name}` does not take a `borrow<{resource}>` first"
                ));
            }
        }
        if calls < 2 {
            problems.push(format!("`{resource}` has no call beside `{open}`"));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// `nvs_stdlib::ext_record` reads and builds exactly the classes the host crosses, and a record
/// whose fields make no value of its class is refused before an instance is made.
#[test]
fn ext_record_crosses_the_hosts_classes_and_refuses_a_record_no_member_could_make() {
    use nvs_stdlib::ext_record::{self, Part};

    let host: Vec<&str> = CORE_CLASSES.iter().map(|record| record.class).collect();
    assert_eq!(ext_record::CLASSES, host.as_slice());

    let refused = [
        (
            "Core\\Time\\Date",
            vec![Part::Int(2024), Part::Int(2), Part::Int(30)],
        ),
        (
            "Core\\Time\\Zone",
            vec![Part::String("Shop/Nowhere".to_owned())],
        ),
        (
            "Core\\Time\\Instant",
            vec![Part::Int(0), Part::Int(1_000_000_000)],
        ),
        ("Core\\Uri", vec![Part::String("a b".to_owned())]),
        ("Core\\Crypto\\PublicKey", vec![Part::Bytes(vec![1, 2, 3])]),
        ("Core\\Uuid", vec![Part::Int(1), Part::Int(2)]),
        ("Core\\Decimal", vec![Part::String("1.5".to_owned())]),
    ];
    for (class, parts) in refused {
        assert!(
            ext_record::built(class, parts).is_err(),
            "{class} was built"
        );
    }
}
