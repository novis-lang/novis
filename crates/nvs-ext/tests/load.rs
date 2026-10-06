//! `rule:packaging/extension-loading-is-root-controlled`'s load refusals, each naming the entry: the
//! pin, the component, the manifest against the component's exports, the imports against the world,
//! the reserved namespaces, a source file outside the extension's namespace, a class two entries
//! declare, and the world's version.

use std::path::PathBuf;

use nvs_ext::call::Host;
use nvs_ext::load::{Entry, Loader, Refused, Set, WORLD_IMPORTS, pin};
use nvs_ext::manifest::WorldVersion;
use nvs_ext::pack::append_section;
use nvs_ext::section::{MANIFEST, SOURCE};
use wasmtime::Engine;

/// A component exporting `shop:geo/api` with one function,
/// `distance-km: func(from: string, round: bool) -> result<f64, error>`, its `error` imported from
/// `nvs:ext/types`. `extra` is spliced in as further top-level imports.
fn guest(extra: &str) -> Vec<u8> {
    let text = format!(
        r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  {extra}
  (core module $m
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
    (func (export "distance-km") (param i32 i32 i32) (result i32) i32.const 16))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "realloc" (core func $realloc))
  (func $f (param "from" string) (param "round" bool) (result (result f64 (error $error)))
    (canon lift (core func $i "distance-km") (memory $mem) (realloc $realloc)))
  (instance $api (export "distance-km" (func $f)))
  (export "shop:geo/api" (instance $api)))"#
    );
    wat::parse_str(text).expect("the test component compiles")
}

const METHOD: &str = r#"{"name": "distanceKm", "params": [{"name": "from", "type": "string"}, {"name": "round", "type": "bool"}], "returns": "float"}"#;

/// The manifest declaring `class` with `methods`, built against `world`.
fn manifest(class: &str, world: &str, methods: &str) -> String {
    let class = class.replace('\\', "\\\\");
    format!(
        r#"{{"manifest": 1, "world": "{world}", "class": "{class}", "interface": "shop:geo/api", "methods": [{methods}]}}"#
    )
}

const SOURCE_JSON: &str =
    r#"{"source": 1, "files": [{"path": "Geo/Units.nvs", "text": "namespace Shop\\Geo;\n"}]}"#;

/// `component` with `manifest` and the one source file added.
fn nvsx(component: Vec<u8>, manifest: &str) -> Vec<u8> {
    let bytes = append_section(component, MANIFEST, manifest.as_bytes());
    append_section(bytes, SOURCE, SOURCE_JSON.as_bytes())
}

fn good(class: &str) -> Vec<u8> {
    nvsx(guest(""), &manifest(class, "1.0.0", METHOD))
}

fn entry(name: &str, bytes: &[u8]) -> Entry {
    Entry {
        path: PathBuf::from(name),
        sha256: pin(bytes),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
    }
}

fn loader() -> Loader {
    Loader::new(&Engine::default())
}

/// The refusal of `bytes` under its own pin, as the entry `geo.nvsx`.
fn refusal(loader: &Loader, bytes: &[u8]) -> Refused {
    let err = loader
        .load_bytes(&entry("geo.nvsx", bytes), bytes)
        .expect_err("the entry is refused");
    assert_eq!(err.entry, "geo.nvsx", "{err}");
    err
}

fn assert_says(err: &Refused, words: &[&str]) {
    for word in words {
        assert!(err.reason.contains(word), "`{word}` is not in: {err}");
    }
}

#[test]
fn a_pinned_component_with_a_matching_manifest_loads() {
    let bytes = good("Shop\\Geo");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("nvs-ext-load");
    std::fs::create_dir_all(&dir).expect("the scratch directory is made");
    let path = dir.join(format!("geo-{}.nvsx", std::process::id()));
    std::fs::write(&path, &bytes).expect("the file is written");
    let loaded = loader().load(&Entry {
        path: path.clone(),
        sha256: pin(&bytes).to_uppercase(),
        memory: None,
        grants: nvs_config::extension::Granted::default(),
    });
    std::fs::remove_file(&path).expect("the file is removed");
    let extension = loaded.expect("the extension loads");
    assert_eq!(extension.manifest.class, "Shop\\Geo");
    assert_eq!(extension.sha256, pin(&bytes));
    assert_eq!(extension.source.files.len(), 1);
}

#[test]
fn a_file_whose_digest_differs_from_its_pin_is_refused_naming_the_entry() {
    let bytes = good("Shop\\Geo");
    let mut wrong = entry("geo.nvsx", &bytes);
    wrong.sha256 = "0".repeat(64);
    let err = loader()
        .load_bytes(&wrong, &bytes)
        .expect_err("the digest differs");
    assert_eq!(err.entry, "geo.nvsx");
    assert_says(&err, &["sha256", &pin(&bytes), &"0".repeat(64)]);
    assert!(err.to_string().contains("`geo.nvsx`"), "{err}");
}

#[test]
fn a_file_that_is_not_a_valid_component_is_refused() {
    let module = wat::parse_str("(module)").expect("the module compiles");
    assert_says(&refusal(&loader(), &module), &["not a valid component"]);
    assert_says(
        &refusal(&loader(), b"not wasm at all"),
        &["not a valid component"],
    );
}

#[test]
fn a_component_without_a_manifest_section_is_refused() {
    let bytes = append_section(guest(""), SOURCE, SOURCE_JSON.as_bytes());
    assert_says(&refusal(&loader(), &bytes), &["no `nvs.manifest`"]);
}

#[test]
fn a_malformed_manifest_is_refused() {
    let bytes = nvsx(guest(""), r#"{"manifest": 1, "world": "1.0.0"}"#);
    assert_says(&refusal(&loader(), &bytes), &["manifest"]);
    let bytes = nvsx(guest(""), "not json");
    assert_says(&refusal(&loader(), &bytes), &["not a JSON object"]);
}

#[test]
fn a_manifest_signature_its_exports_do_not_match_is_refused() {
    let cases = [
        (
            r#"{"name": "distanceKm", "params": [{"name": "from", "type": "int"}, {"name": "round", "type": "bool"}], "returns": "float"}"#,
            "the parameter `from`",
        ),
        (
            r#"{"name": "distanceKm", "params": [{"name": "from", "type": "string"}], "returns": "float"}"#,
            "1 parameter(s)",
        ),
        (
            r#"{"name": "distanceKm", "params": [{"name": "from", "type": "string"}, {"name": "round", "type": "bool"}], "returns": "string"}"#,
            "returns `string`",
        ),
        (
            r#"{"name": "distanceKm", "params": [{"name": "start", "type": "string"}, {"name": "round", "type": "bool"}], "returns": "float"}"#,
            "`start`",
        ),
        (r#"{"name": "area"}"#, "no function `area`"),
    ];
    for (method, says) in cases {
        let bytes = nvsx(guest(""), &manifest("Shop\\Geo", "1.0.0", method));
        assert_says(&refusal(&loader(), &bytes), &[says]);
    }
    let other =
        r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo", "interface": "shop:geo/other"}"#;
    assert_says(
        &refusal(&loader(), &nvsx(guest(""), other)),
        &["no instance `shop:geo/other`"],
    );
}

#[test]
fn a_manifest_naming_a_type_outside_the_value_table_is_refused() {
    for ty in ["object", "callable", "array<string", "int|string"] {
        let method = format!(
            r#"{{"name": "distanceKm", "params": [{{"name": "from", "type": "{ty}"}}, {{"name": "round", "type": "bool"}}], "returns": "float"}}"#
        );
        let bytes = nvsx(guest(""), &manifest("Shop\\Geo", "1.0.0", &method));
        assert_says(
            &refusal(&loader(), &bytes),
            &[&format!(
                "the type `{ty}` is not one an extension signature can carry"
            )],
        );
    }
}

/// `distanceKm` with its `from` parameter typed `from` and `extra` keys spliced into the method.
fn method_with(from: &str, extra: &str) -> String {
    format!(
        r#"{{"name": "distanceKm", "params": [{{"name": "from", "type": "{from}"}}, {{"name": "round", "type": "bool"}}], "returns": "float"{extra}}}"#
    )
}

#[test]
fn a_manifest_declaring_a_secret_return_is_refused() {
    let cases = [
        (
            method_with("string", r#", "secret": true"#),
            "the return of `distanceKm` is declared `secret`",
        ),
        (
            method_with("string", "").replace(r#""float""#, r#""secret float""#),
            "the return of `distanceKm` is declared `secret`",
        ),
        (
            method_with("secret string", ""),
            "the parameter `from` of `distanceKm` is declared `secret`",
        ),
        (
            method_with("string", "").replace(r#""float""#, r#""tainted float""#),
            "writes `tainted` in its type",
        ),
    ];
    for (method, says) in cases {
        let bytes = nvsx(guest(""), &manifest("Shop\\Geo", "1.0.0", &method));
        assert_says(&refusal(&loader(), &bytes), &[says]);
    }
}

#[test]
fn a_manifest_naming_an_unknown_qualifier_is_refused() {
    let on_method = method_with("string", r#", "trusted": true"#);
    let on_param = method_with("string", "").replace(
        r#""type": "string"}"#,
        r#""type": "string", "launders": "html"}"#,
    );
    for (method, key) in [(on_method, "trusted"), (on_param, "launders")] {
        let bytes = nvsx(guest(""), &manifest("Shop\\Geo", "1.0.0", &method));
        assert_says(
            &refusal(&loader(), &bytes),
            &[&format!("unknown field `{key}`")],
        );
    }
    let declared = method_with("string", r#", "source": true"#)
        .replace(r#""type": "string"}"#, r#""type": "string", "sink": true}"#);
    let bytes = nvsx(guest(""), &manifest("Shop\\Geo", "1.0.0", &declared));
    loader()
        .load_bytes(&entry("geo.nvsx", &bytes), &bytes)
        .expect("a sink and a source are the declarations a manifest may make");
}

#[test]
fn an_import_outside_the_world_is_refused_naming_the_import() {
    for import in [
        "wasi:sockets/tcp@0.2.12",
        "wasi:http/types@0.2.12",
        "shop:other/api",
        "wasi:cli/stdout@0.3.0",
    ] {
        let component = guest(&format!(r#"(import "{import}" (instance))"#));
        let bytes = nvsx(component, &manifest("Shop\\Geo", "1.0.0", METHOD));
        assert_says(
            &refusal(&loader(), &bytes),
            &[&format!("imports `{import}`")],
        );
    }
    let component = guest(r#"(import "wasi:cli/stdout@0.2.3" (instance))"#);
    let bytes = nvsx(component, &manifest("Shop\\Geo", "1.0.0", METHOD));
    loader()
        .load_bytes(&entry("geo.nvsx", &bytes), &bytes)
        .expect("an older WASI 0.2 import is in the world");
}

#[test]
fn a_component_importing_a_linked_wasi_interface_loads() {
    let import = r#"(import "wasi:cli/environment@0.2.12" (instance
    (export "get-arguments" (func (result (list string))))))"#;
    let bytes = nvsx(guest(import), &manifest("Shop\\Geo", "1.0.0", METHOD));
    Host::new(1, |linker| {
        let extension = Loader::new(linker.engine())
            .load_bytes(&entry("geo.nvsx", &bytes), &bytes)
            .expect("a WASI interface the host links loads");
        linker
            .instantiate_pre(&extension.component)
            .expect("the host's linker defines what the component imports");
        Ok(())
    })
    .expect("the host starts");
}

#[test]
fn the_world_imports_are_the_wit_world_s() {
    let world = include_str!("../../../wit/nvs-ext/world.wit");
    let wit: Vec<String> = world
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import "))
        .map(|name| {
            let name = name.trim_end_matches(';');
            let name = name.split_once('@').map_or(name, |(bare, _)| bare);
            if name.contains(':') {
                name.to_owned()
            } else {
                format!("nvs:ext/{name}")
            }
        })
        .collect();
    assert_eq!(wit, WORLD_IMPORTS);
}

#[test]
fn a_class_under_core_or_novis_is_refused() {
    for class in ["Core\\Geo", "Novis\\Geo", "core\\Geo"] {
        let bytes = good(class);
        assert_says(
            &refusal(&loader(), &bytes),
            &[&format!("the class `{class}`")],
        );
    }
}

#[test]
fn a_source_file_outside_the_extension_namespace_is_refused_naming_the_file() {
    let with_source = |text: &str| {
        let files = serde_json::json!({"source": 1, "files": [
            {"path": "Geo/Units.nvs", "text": "namespace Shop\\Geo;\n"},
            {"path": "Geo/Map.nvs", "text": text},
        ]});
        let bytes = append_section(
            guest(""),
            MANIFEST,
            manifest("Shop\\Geo", "1.0.0", METHOD).as_bytes(),
        );
        append_section(bytes, SOURCE, files.to_string().as_bytes())
    };
    for (text, says) in [
        ("namespace Blog\\Geo;\n", "declares `namespace Blog\\Geo`"),
        ("namespace Shopping;\n", "declares `namespace Shopping`"),
        ("class Map {}\n", "declares no namespace"),
        ("namespace Shop\\Geo {}\n", "declares no namespace"),
    ] {
        assert_says(
            &refusal(&loader(), &with_source(text)),
            &["the source file `Geo/Map.nvs`", says, "`Shop`"],
        );
    }
    for text in [
        "namespace Shop;\n",
        "namespace shop\\Geo\\Map;\n",
        "<?nvs\n// A map.\n# More.\n/* Still a comment. */\nnamespace Shop\\Geo;\nclass Map {}\n",
    ] {
        let bytes = with_source(text);
        loader()
            .load_bytes(&entry("geo.nvsx", &bytes), &bytes)
            .unwrap_or_else(|err| panic!("`{text}` loads: {err}"));
    }
}

#[test]
fn a_class_another_loaded_extension_declares_is_refused() {
    let first = good("Shop\\Geo");
    let second = nvsx(
        guest(r#"(import "nvs:ext/log@1.0.0" (instance))"#),
        &manifest("shop\\geo", "1.0.0", METHOD),
    );
    let loader = loader();
    let mut set = Set::default();
    set.insert(
        loader
            .load_bytes(&entry("geo.nvsx", &first), &first)
            .expect("the first loads"),
    )
    .expect("the first joins the set");
    let err = set
        .insert(
            loader
                .load_bytes(&entry("geo-copy.nvsx", &second), &second)
                .expect("the second loads alone"),
        )
        .expect_err("the class is taken");
    assert_eq!(err.entry, "geo-copy.nvsx");
    assert_says(&err, &["`shop\\geo`", "`geo.nvsx`"]);
    assert_eq!(set.extensions().len(), 1);
}

#[test]
fn a_component_built_against_a_newer_minor_or_another_major_is_refused_naming_both_versions() {
    let host = loader().with_world(WorldVersion {
        major: 1,
        minor: 2,
        patch: 0,
    });
    for built in ["1.3.0", "2.0.0", "0.9.0"] {
        let bytes = nvsx(guest(""), &manifest("Shop\\Geo", built, METHOD));
        assert_says(
            &refusal(&host, &bytes),
            &[&format!("nvs:ext@{built}"), "nvs:ext@1.2.0"],
        );
    }
}

#[test]
fn a_component_built_against_an_older_minor_loads() {
    let host = loader().with_world(WorldVersion {
        major: 1,
        minor: 2,
        patch: 0,
    });
    let component = guest(r#"(import "nvs:ext/log@1.1.4" (instance))"#);
    let bytes = nvsx(component, &manifest("Shop\\Geo", "1.1.4", METHOD));
    host.load_bytes(&entry("geo.nvsx", &bytes), &bytes)
        .expect("an older minor loads");
}
