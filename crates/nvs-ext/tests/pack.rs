//! The packer: a component or a core module, a manifest and source files make a `.nvsx` that loads,
//! the same bytes each time, and a source path that leaves the project is refused.

use std::path::PathBuf;

use nvs_ext::load::{Entry, Extension, Loader, pin};
use nvs_ext::pack::{Inputs, pack};
use nvs_ext::section;
use nvs_ext::source::{Source, SourceFile};
use wasmtime::Engine;

/// The author's WIT: the interface the manifest names.
const WIT: &str = r"package shop:geo;

interface api {
    use nvs:ext/types@1.0.0.{error};

    distance-km: func(%from: string, round: bool) -> result<f64, error>;
}
";

const MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo", "interface": "shop:geo/api", "methods": [{"name": "distanceKm", "params": [{"name": "from", "type": "string"}, {"name": "round", "type": "bool"}], "returns": "float"}]}"#;

/// A component exporting `shop:geo/api`, written by hand as a bindings generator would write it.
fn component() -> Vec<u8> {
    wat::parse_str(
        r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
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
  (export "shop:geo/api" (instance $api)))"#,
    )
    .expect("the test component compiles")
}

/// The same function as a core module, its exports named by the canonical ABI.
fn core_module() -> Vec<u8> {
    wat::parse_str(
        r#"(module
  (memory (export "memory") 1)
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
  (func (export "shop:geo/api#distance-km") (param i32 i32 i32) (result i32) i32.const 16))"#,
    )
    .expect("the test module compiles")
}

fn files() -> Vec<SourceFile> {
    vec![
        SourceFile {
            path: "Geo/Units.nvs".to_owned(),
            text: "namespace Shop\\Geo;\n\nconst KM_PER_MILE = 1.609344;\n".to_owned(),
        },
        SourceFile {
            path: "Geo/Format.nvs".to_owned(),
            text: "namespace Shop\\Geo;\n// \"quoted\" \u{e9}\t\n".to_owned(),
        },
    ]
}

fn packed(wasm: &[u8], files: &[SourceFile]) -> Vec<u8> {
    pack(&Inputs {
        wasm,
        wit: &[("geo.wit", WIT)],
        manifest: MANIFEST.as_bytes(),
        files,
    })
    .expect("the inputs pack")
}

fn load(bytes: &[u8]) -> Extension {
    Loader::new(&Engine::default())
        .load_bytes(
            &Entry {
                path: PathBuf::from("geo.nvsx"),
                sha256: pin(bytes),
                memory: None,
                grants: nvs_config::extension::Granted::default(),
            },
            bytes,
        )
        .expect("the packed file loads")
}

#[test]
fn a_component_and_a_manifest_pack_into_a_file_that_loads() {
    let extension = load(&packed(&component(), &files()));
    assert_eq!(extension.manifest.class, "Shop\\Geo");
    assert_eq!(extension.source.files, files());
}

#[test]
fn a_core_module_is_componentized_against_the_world_and_loads() {
    let module = core_module();
    assert!(wasmparser::Parser::is_core_wasm(&module));
    let bytes = packed(&module, &files());
    assert!(wasmparser::Parser::is_component(&bytes));
    let extension = load(&bytes);
    assert_eq!(extension.manifest.interface, "shop:geo/api");
}

#[test]
fn the_packed_file_carries_the_manifest_and_the_source_files_byte_for_byte() {
    for wasm in [component(), core_module()] {
        let bytes = packed(&wasm, &files());
        let sections = section::read(&bytes).expect("the sections read");
        assert_eq!(sections.manifest, Some(MANIFEST.as_bytes()));
        let source = Source::parse(sections.source.expect("the source section is there"))
            .expect("the source section parses");
        assert_eq!(source.files, files());
    }
}

#[test]
fn packing_the_same_inputs_twice_gives_the_same_bytes() {
    for wasm in [component(), core_module()] {
        let first = packed(&wasm, &files());
        let second = packed(&wasm, &files());
        assert_eq!(first, second);
        assert_eq!(pin(&first), pin(&second));
    }
}

#[test]
fn a_source_path_that_leaves_the_project_is_refused_by_the_packer() {
    for path in [
        "../Secret.nvs",
        "Geo/../../Secret.nvs",
        "/etc/Secret.nvs",
        "C:/Secret.nvs",
        "Geo\\Secret.nvs",
    ] {
        let files = [SourceFile {
            path: path.to_owned(),
            text: "namespace Shop\\Geo;\n".to_owned(),
        }];
        let err = pack(&Inputs {
            wasm: &component(),
            wit: &[],
            manifest: MANIFEST.as_bytes(),
            files: &files,
        })
        .expect_err("the path is refused");
        assert!(err.0.contains(&format!("`{path}`")), "{err}");
    }
}
