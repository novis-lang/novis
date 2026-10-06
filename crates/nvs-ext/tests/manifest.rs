//! `rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`'s two sections, read without the
//! engine: where they are found in a component, and what a manifest and a source payload must be to
//! read at all. Whether a manifest matches its component's exports is `tests/load.rs`'s.

use nvs_ext::manifest::{Manifest, SettingType, WorldVersion};
use nvs_ext::section::{self, MANIFEST, SOURCE};
use nvs_ext::source::Source;

/// `bytes` with a custom section `name` holding `data` appended. A custom section is valid at any
/// point of a component's top level, so the end is as good as anywhere.
fn with_section(mut bytes: Vec<u8>, name: &str, data: &[u8]) -> Vec<u8> {
    fn leb(out: &mut Vec<u8>, mut n: usize) {
        loop {
            let byte = u8::try_from(n & 0x7f).expect("seven bits");
            n >>= 7;
            if n == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }
    let mut body = Vec::new();
    leb(&mut body, name.len());
    body.extend_from_slice(name.as_bytes());
    body.extend_from_slice(data);
    bytes.push(0);
    leb(&mut bytes, body.len());
    bytes.extend(body);
    bytes
}

fn component(text: &str) -> Vec<u8> {
    wat::parse_str(text).expect("the test component compiles")
}

const MINIMAL: &str =
    r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo", "interface": "shop:geo/api"}"#;

fn manifest(json: &str) -> Result<Manifest, String> {
    Manifest::parse(json.as_bytes()).map_err(|err| err.0)
}

#[test]
fn both_sections_are_found_at_the_top_level_of_a_component() {
    let bytes = with_section(component("(component)"), MANIFEST, MINIMAL.as_bytes());
    let bytes = with_section(bytes, SOURCE, br#"{"source": 1}"#);
    let sections = section::read(&bytes).expect("the sections read");

    assert_eq!(sections.manifest, Some(MINIMAL.as_bytes()));
    assert_eq!(sections.source, Some(&br#"{"source": 1}"#[..]));
}

#[test]
fn a_section_inside_a_nested_module_is_not_the_extensions() {
    let text = r#"(component (core module (@custom "nvs.manifest" "{}")))"#;
    let bytes = component(text);
    assert!(
        bytes
            .windows(MANIFEST.len())
            .any(|w| w == MANIFEST.as_bytes()),
        "the nested module carries the section"
    );
    let sections = section::read(&bytes).expect("the component reads");

    assert_eq!(sections.manifest, None);
}

#[test]
fn a_core_module_a_repeated_section_and_bytes_that_are_not_wasm_are_refused() {
    let module = with_section(component("(module)"), MANIFEST, MINIMAL.as_bytes());
    let twice = with_section(
        with_section(component("(component)"), SOURCE, b"{}"),
        SOURCE,
        b"{}",
    );

    let said = |bytes: &[u8]| section::read(bytes).expect_err("refused").0;
    assert!(said(&module).contains("a WebAssembly module, not a component"));
    assert!(said(&twice).contains("`nvs.source` appears twice"));
    assert!(said(b"PK\x03\x04").contains("not WebAssembly"));
}

#[test]
fn a_full_manifest_reads_with_its_qualifiers_settings_and_requests() {
    let json = r#"{
        "manifest": 1,
        "world": "1.2.3",
        "class": "Shop\\Geo",
        "interface": "shop:geo/api",
        "methods": [{
            "name": "distanceKm",
            "params": [{"name": "from", "type": "string", "sink": true},
                       {"name": "round", "type": "bool", "default": false}],
            "returns": "float",
            "source": true,
            "help": "The distance between two places."
        }, {"name": "reset"}],
        "consts": [{"name": "RADIUS", "type": "float", "value": 6371.0}],
        "settings": {"name": "geo", "keys": [{"name": "zones", "type": "array<string>", "default": ["eu"]}]},
        "requests": {"read": ["data/geo/"], "connect": ["tiles.example.com"]},
        "memory": 67108864
    }"#;
    let m = manifest(json).expect("the manifest reads");

    assert_eq!(
        m.world,
        WorldVersion {
            major: 1,
            minor: 2,
            patch: 3
        }
    );
    assert_eq!(m.methods[0].export_name(), "distance-km");
    assert!(m.methods[0].params[0].sink && m.methods[0].source);
    assert_eq!(m.methods[1].returns, "void");
    assert_eq!(
        m.settings.as_ref().map(|s| s.keys[0].ty),
        Some(SettingType::Strings)
    );
    assert_eq!(m.requests.connect, ["tiles.example.com"]);
    assert!(m.requests.write.is_empty());
    assert_eq!(m.memory, Some(64 << 20));
}

#[test]
fn a_newer_format_is_refused_by_its_number_before_its_keys() {
    let said = manifest(r#"{"manifest": 2, "anything": "new"}"#).expect_err("refused");
    assert!(said.contains("format 2"), "{said}");

    let said = manifest(r#"{"world": "1.0.0"}"#).expect_err("refused");
    assert!(said.contains("no `manifest` format number"), "{said}");
}

#[test]
fn a_manifest_that_does_not_read_is_refused_naming_what_is_wrong() {
    let cases = [
        (MINIMAL.replace("}", r#", "extra": 1}"#), "unknown field `extra`"),
        (MINIMAL.replace("1.0.0", "1.0"), "`1.0` is not a version"),
        (MINIMAL.replace(r"Shop\\Geo", r"Shop\\"), "is not a class name"),
        (MINIMAL.replace("shop:geo/api", "Geo"), "`namespace:package/interface`"),
        (
            MINIMAL.replace("}", r#", "methods": [{"name": "a"}, {"name": "a"}]}"#),
            "the method `a` is declared twice",
        ),
        (
            MINIMAL.replace(
                "}",
                r#", "methods": [{"name": "a", "params": [{"name": "x-y", "type": "int"}]}]}"#,
            ),
            "the parameter of `a` `x-y` is not a name",
        ),
        (
            MINIMAL.replace(
                "}",
                r#", "settings": {"name": "geo", "keys": [{"name": "n", "type": "uint", "default": -1}]}}"#,
            ),
            "the setting `n` has the default -1",
        ),
        (
            MINIMAL.replace(
                "}",
                r#", "settings": {"name": "geo", "keys": [{"name": "n", "type": "decimal", "default": 1}]}}"#,
            ),
            "unknown variant `decimal`",
        ),
    ];

    for (json, wanted) in cases {
        let said = manifest(&json).expect_err(&json);
        assert!(said.contains(wanted), "{json}\n-- said: {said}");
    }
}

#[test]
fn source_files_are_relative_nvs_paths_named_once() {
    let parse = |json: &str| Source::parse(json.as_bytes()).map_err(|err| err.0);
    let read = parse(
        r#"{"source": 1, "files": [{"path": "Geo/Units.nvs", "text": "namespace Shop\\Geo;"}]}"#,
    )
    .expect("the source reads");
    assert_eq!(read.files[0].path, "Geo/Units.nvs");

    for path in [
        "../Units.nvs",
        "/Units.nvs",
        "Geo//Units.nvs",
        "Geo\\\\Units.nvs",
        "C:Units.nvs",
        "Units.txt",
    ] {
        let said = parse(&format!(
            r#"{{"source": 1, "files": [{{"path": "{path}", "text": ""}}]}}"#
        ))
        .expect_err(path);
        assert!(said.contains("not a relative path"), "{path}: {said}");
    }

    let said = parse(
        r#"{"source": 1, "files": [{"path": "a.nvs", "text": ""}, {"path": "a.nvs", "text": ""}]}"#,
    )
    .expect_err("a path twice");
    assert!(said.contains("appears twice"), "{said}");
    let said = parse(r#"{"source": 3}"#).expect_err("a newer format");
    assert!(said.contains("format 3"), "{said}");
}
