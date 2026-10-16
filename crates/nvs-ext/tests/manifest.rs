//! `rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`'s two sections, read without the
//! engine: where they are found in a component, and what a manifest and a source payload must be to
//! read at all. Whether a manifest matches its component's exports is `tests/load.rs`'s.

use nvs_ext::manifest::{Manifest, SettingType, WorldVersion};
use nvs_ext::section::{self, MANIFEST, SOURCE};
use nvs_ext::source::{Source, SourceFile};

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

/// `rule:security/image-component-declares-nothing`: the image manifest marks no method a `source`
/// or `secret` and no parameter a `sink`, and requests no file and no host, so a thumbnail keeps
/// its input's qualifiers by ordinary contagion and the component holds no I/O of its own.
#[test]
fn the_image_manifest_declares_no_qualifier_and_no_capability() {
    let path = nvs_repo::path("extensions/image/manifest.json");
    let bytes = std::fs::read(&path).expect("the image manifest is on disk");
    let m = Manifest::parse(&bytes).expect("the image manifest reads");

    let qualified: Vec<String> = m
        .methods
        .iter()
        .flat_map(|method| {
            let own = [("source", method.source), ("secret", method.secret)]
                .into_iter()
                .filter(|(_, set)| *set)
                .map(|(what, _)| format!("{}: {what}", method.name));
            let sinks = method
                .params
                .iter()
                .filter(|param| param.sink)
                .map(|param| format!("{}({}): sink", method.name, param.name));
            own.chain(sinks).collect::<Vec<_>>()
        })
        .collect();
    assert!(qualified.is_empty(), "qualifier deviations: {qualified:?}");
    assert!(m.requests.read.is_empty(), "{:?}", m.requests.read);
    assert!(m.requests.write.is_empty(), "{:?}", m.requests.write);
    assert!(m.requests.connect.is_empty(), "{:?}", m.requests.connect);
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

/// A manifest of `Shop\Geo` declaring `enums`, written as JSON.
fn with_enums(enums: &str) -> String {
    format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo", "interface": "shop:geo/api", "enums": [{enums}]}}"#
    )
}

fn file(path: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: String::new(),
    }
}

#[test]
fn a_manifest_enum_is_keyed_under_its_owner() {
    let read = manifest(&with_enums(
        r#"{"name": "Unit", "owner": "Map", "cases": ["Metres"]},
           {"name": "Shape", "cases": ["Round"]},
           {"name": "Datum", "owner": "Geo", "cases": ["Wgs84"]}"#,
    ))
    .expect("the manifest reads");
    let names: Vec<String> = read.enums.iter().map(|e| read.enum_name(e)).collect();
    assert_eq!(
        names,
        ["Shop\\Map::Unit", "Shop\\Shape", "Shop\\Geo::Datum"]
    );
    assert_eq!(read.enums[0].owner.as_deref(), Some("Map"));
    assert!(
        read.novis_type("{unit: Unit}").is_ok(),
        "a signature writes an owned enum by its name alone"
    );
}

#[test]
fn an_enum_owner_the_manifest_does_not_declare_is_refused() {
    let owned_by = |owner: &str| {
        manifest(&with_enums(&format!(
            r#"{{"name": "Unit", "owner": "{owner}", "cases": ["Metres"]}}"#
        )))
    };
    let read = owned_by("Map").expect("the manifest reads without its source");
    let said = read.check_owners(&[]).expect_err("no class `Map`").0;
    assert!(said.contains("`Map`") && said.contains("`Unit`"), "{said}");
    let said = read
        .check_owners(&[file("Maps.nvs")])
        .expect_err("`Maps` is another class")
        .0;
    assert!(said.contains("`Map`"), "{said}");
    read.check_owners(&[file("Map.nvs")])
        .expect("a source file declares `Map`");
    owned_by("Geo")
        .expect("the manifest's class owns")
        .check_owners(&[])
        .expect("the manifest declares `Geo`");
    owned_by(r"Tiles\\Map")
        .expect("a nested owner reads")
        .check_owners(&[file("Tiles/Map.nvs")])
        .expect("`Tiles/Map.nvs` declares `Tiles\\Map`");

    let said = owned_by("not a class").expect_err("an owner that is no name");
    assert!(said.contains("is not a class name"), "{said}");
}

/// The text of `source` with its comments and string literals left out, so a word the help of a
/// method or a printed message says is not a use.
fn code_of(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(c) = rest.chars().next() {
        if rest.starts_with("//") {
            rest = rest.find('\n').map_or("", |end| &rest[end..]);
        } else if rest.starts_with("/*") {
            rest = rest.find("*/").map_or("", |end| &rest[end + 2..]);
        } else if c == '"' || c == '\'' {
            let mut chars = rest.char_indices().skip(1);
            let mut end = rest.len();
            while let Some((at, next)) = chars.next() {
                if next == '\\' {
                    chars.next();
                } else if next == c {
                    end = at + 1;
                    break;
                }
            }
            out.push(' ');
            rest = &rest[end..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// Whether `code` names `word` as a type: the whole word, and not after `::`, `->` or `$`,
/// which is where a case, a property or a variable of the same spelling is.
fn names_type(code: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    code.match_indices(word).any(|(at, _)| {
        let before = &code[..at];
        let after = &code[at + word.len()..];
        !before.ends_with(is_ident)
            && !after.starts_with(is_ident)
            && !before.ends_with("::")
            && !before.ends_with("->")
            && !before.ends_with('$')
    })
}

/// `rule:enums/an-enum-lives-in-the-class-that-uses-it`, over the built-in components: an enum
/// one source class names is owned by that class, and one several name, or none, is owned by
/// none. The manifest's own class is the binding every source class calls, so its signatures
/// name every enum and are not a use.
#[test]
fn every_component_enum_one_class_uses_is_owned_by_it() {
    let mut wrong = Vec::new();
    for component in ["image", "intl"] {
        let root = nvs_repo::path(&format!("extensions/{component}"));
        let bytes = std::fs::read(root.join("manifest.json")).expect("the manifest is on disk");
        let read = Manifest::parse(&bytes).expect("the manifest reads");
        let mut classes: Vec<(String, String)> = std::fs::read_dir(root.join("nvs"))
            .expect("the component carries source")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "nvs"))
            .map(|path| {
                let class = path
                    .file_stem()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                let text = std::fs::read_to_string(&path).expect("the source reads");
                (class, code_of(&text))
            })
            .collect();
        classes.sort();
        let files: Vec<SourceFile> = classes
            .iter()
            .map(|(class, _)| file(&format!("{class}.nvs")))
            .collect();
        if let Err(err) = read.check_owners(&files) {
            wrong.push(format!("{component}: {}", err.0));
        }
        for decl in &read.enums {
            let users: Vec<&str> = classes
                .iter()
                .filter(|(class, code)| match &decl.owner {
                    Some(owner) if owner == class => {
                        names_type(code, &decl.name)
                            || code.contains(&format!("{owner}::{}", decl.name))
                    }
                    Some(owner) => code.contains(&format!("{owner}::{}", decl.name)),
                    None => names_type(code, &decl.name),
                })
                .map(|(class, _)| class.as_str())
                .collect();
            let want = match users.as_slice() {
                [one] => Some(*one),
                _ => None,
            };
            if decl.owner.as_deref() != want {
                wrong.push(format!(
                    "{}: owned by {:?}, used by {users:?}",
                    read.enum_name(decl),
                    decl.owner
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "misplaced component enums:\n{}",
        wrong.join("\n")
    );
}
