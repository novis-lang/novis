//! The fixture extensions under `tests/conformance/ext/fixtures/`, which conformance cases load:
//! each committed `<name>.nvsx` is exactly what the packer builds from the directory `<name>/`
//! beside it, so a fixture never goes stale against its text.
//!
//! A fixture directory holds `module.wat`, a core module whose exports follow the canonical ABI's
//! names; `api.wit`, the author's WIT package; `manifest.json`; and `source/`, the Novis files of
//! its `nvs.source` section, each packed under its path there less a trailing `.src`. The packer componentizes the module
//! against the world, as `nvs ext build` does for an author's build. That is why a fixture is a
//! core module and not component text: `wit-component` writes the component's types, which are
//! the part a hand-written component gets wrong.
//!
//! Beside each `<name>.nvsx` is `<name>.sha256`, its pin as one line of hex. `nvs-test` reads it
//! to write the `[[extension]]` entry a case's `--EXTENSION--` section asks for, because that
//! crate is std-only and has no hash of its own; this test holds the pin to the bytes too.
//!
//! A change to a fixture's text, or to the packer, the world or `wit-component`, changes the bytes.
//! `NVS_WRITE_FIXTURES=1 cargo test --test fixtures` writes them again, and the diff is reviewed
//! like any other.
//!
//! `Shop\Ledger` is the fixture the compiler goal's cases call. Its `echo*` methods return their
//! argument unchanged, one per row of `rule:packaging/a-value-crosses-as-its-wit-type`'s table the
//! loader reads; `writeLine` takes a sink parameter, `fetch` returns a source, `fail` returns each
//! `err` case and `crash` traps.

use std::path::{Path, PathBuf};

use nvs_ext::load::{Entry, Extension, Loader, pin};
use nvs_ext::pack::{Inputs, pack};
use nvs_ext::source::SourceFile;
use wasmtime::component::{Linker, Val};
use wasmtime::{Engine, Store};

/// What a fixture's source file adds to the path it is packed under. A `.nvs` file under
/// `tests/conformance/` would be run as a program beside the `.nvst` cases.
const SOURCE_SUFFIX: &str = ".src";

fn fixtures() -> PathBuf {
    nvs_repo::path("tests/conformance/ext/fixtures")
}

/// Every fixture's name: each directory under [`fixtures`].
fn names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures())
        .expect("the fixture directory reads")
        .map(|entry| entry.expect("the fixture directory reads"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} does not read: {err}", path.display()))
        .replace("\r\n", "\n")
}

/// The files under `dir`, their paths relative to `root` with `/` and without [`SOURCE_SUFFIX`],
/// in path order.
fn source_files(root: &Path, dir: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("the source directory reads").path();
        if path.is_dir() {
            source_files(root, &path, out);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let packed = relative.strip_suffix(SOURCE_SUFFIX).unwrap_or_else(|| {
                panic!("the fixture source `{relative}` does not end in `{SOURCE_SUFFIX}`")
            });
            out.push(SourceFile {
                path: packed.to_owned(),
                text: read(&path),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
}

/// The `.nvsx` the packer builds from the fixture `name`'s text.
fn build(name: &str) -> Vec<u8> {
    let dir = fixtures().join(name);
    let module = wat::parse_str(read(&dir.join("module.wat")))
        .unwrap_or_else(|err| panic!("{name}'s module.wat does not compile: {err}"));
    let wit = read(&dir.join("api.wit"));
    let manifest = read(&dir.join("manifest.json"));
    let mut files = Vec::new();
    source_files(&dir.join("source"), &dir.join("source"), &mut files);
    pack(&Inputs {
        wasm: &module,
        wit: &[("api.wit", &wit)],
        manifest: manifest.as_bytes(),
        files: &files,
    })
    .unwrap_or_else(|err| panic!("{name} does not pack: {err}"))
}

fn load(engine: &Engine, name: &str, bytes: &[u8]) -> Extension {
    Loader::new(engine)
        .load_bytes(
            &Entry {
                path: PathBuf::from(format!("{name}.nvsx")),
                sha256: pin(bytes),
                memory: None,
            },
            bytes,
        )
        .unwrap_or_else(|err| panic!("{name} does not load: {err}"))
}

#[test]
fn every_committed_conformance_fixture_is_what_the_packer_builds_from_its_text() {
    let names = names();
    assert!(names.iter().any(|name| name == "ledger"), "{names:?}");
    let write = std::env::var_os("NVS_WRITE_FIXTURES").is_some();
    for name in names {
        let built = build(&name);
        let path = fixtures().join(format!("{name}.nvsx"));
        let pin_path = fixtures().join(format!("{name}.sha256"));
        let pinned = format!("{}\n", pin(&built));
        if write {
            std::fs::write(&path, &built)
                .unwrap_or_else(|err| panic!("{} does not write: {err}", path.display()));
            std::fs::write(&pin_path, &pinned)
                .unwrap_or_else(|err| panic!("{} does not write: {err}", pin_path.display()));
            continue;
        }
        assert_eq!(
            read(&pin_path),
            pinned,
            "{} is not the pin of what the packer builds from `{name}/`; \
             `NVS_WRITE_FIXTURES=1 cargo test --test fixtures` writes it again",
            pin_path.display()
        );
        let committed = std::fs::read(&path).unwrap_or_else(|err| {
            panic!(
                "{} does not read ({err}); `NVS_WRITE_FIXTURES=1 cargo test --test fixtures` writes it",
                path.display()
            )
        });
        assert!(
            committed == built,
            "{} differs from what the packer builds from `{name}/`; \
             `NVS_WRITE_FIXTURES=1 cargo test --test fixtures` writes it again",
            path.display()
        );
    }
}

#[test]
fn the_ledger_fixture_loads_with_its_class_its_methods_and_its_source() {
    let extension = load(&Engine::default(), "ledger", &build("ledger"));
    let manifest = &extension.manifest;
    assert_eq!(manifest.class, "Shop\\Ledger");
    let method = |name: &str| {
        manifest
            .methods
            .iter()
            .find(|method| method.name == name)
            .unwrap_or_else(|| panic!("`{name}` is a method"))
    };
    assert!(method("writeLine").params[0].sink);
    assert!(method("fetch").source);
    assert_eq!(method("crash").returns, "void");
    assert_eq!(manifest.consts[0].name, "CURRENCY");
    let paths: Vec<&str> = extension
        .source
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(paths, ["Ledger/Receipt.nvs"]);
}

/// Calls the ledger's export `name` with `args` and returns its one result.
fn call(name: &str, args: &[Val]) -> Val {
    try_call(name, args).unwrap_or_else(|err| panic!("`{name}` traps: {err:#}"))
}

/// Calls the ledger's export `name` with `args` in a fresh instance, or the trap it stops with.
fn try_call(name: &str, args: &[Val]) -> wasmtime::Result<Val> {
    let engine = Engine::default();
    let extension = load(&engine, "ledger", &build("ledger"));
    let mut linker = Linker::new(&engine);
    linker
        .define_unknown_imports_as_traps(&extension.component)
        .expect("the imports define");
    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &extension.component)
        .expect("the fixture instantiates");
    let api = instance
        .get_export_index(&mut store, None, "shop:ledger/api")
        .expect("the interface is exported");
    let func = instance
        .get_export_index(&mut store, Some(&api), name)
        .and_then(|index| instance.get_func(&mut store, index))
        .unwrap_or_else(|| panic!("`{name}` is exported"));
    let mut results = [Val::Bool(false)];
    func.call(&mut store, args, &mut results)?;
    let [result] = results;
    Ok(result)
}

fn ok(value: Val) -> Val {
    Val::Result(Ok(Some(Box::new(value))))
}

fn string(text: &str) -> Val {
    Val::String(text.to_owned())
}

#[test]
fn every_echo_export_of_the_ledger_returns_its_argument() {
    let item = |note: Option<&str>| {
        Val::Record(vec![
            ("name".to_owned(), string("pen")),
            (
                "note".to_owned(),
                Val::Option(note.map(|n| Box::new(string(n)))),
            ),
        ])
    };
    let rows = [
        ("echo-bool", Val::Bool(true)),
        ("echo-int", Val::S64(-42)),
        ("echo-uint", Val::U64(u64::MAX)),
        ("echo-float", Val::Float64(2.5)),
        ("echo-string", string("caf\u{e9}")),
        ("echo-bytes", Val::List(vec![Val::U8(0), Val::U8(255)])),
        ("echo-list", Val::List(vec![Val::S64(1), Val::S64(2)])),
        (
            "echo-map",
            Val::List(vec![
                Val::Tuple(vec![string("b"), Val::S64(2)]),
                Val::Tuple(vec![string("a"), Val::S64(1)]),
            ]),
        ),
        ("echo-item", item(Some("blue"))),
        ("echo-item", item(None)),
        ("echo-optional", Val::Option(Some(Box::new(Val::S64(7))))),
        ("echo-optional", Val::Option(None)),
    ];
    for (name, value) in rows {
        assert_eq!(
            call(name, std::slice::from_ref(&value)),
            ok(value),
            "{name}"
        );
    }
}

#[test]
fn the_ledger_writes_a_line_fetches_one_and_fails_with_each_error_case() {
    assert_eq!(call("write-line", &[string("x")]), Val::Result(Ok(None)));
    assert_eq!(call("fetch", &[]), ok(string("remote")));
    for (kind, case) in [(0, "invalid"), (1, "parse"), (2, "runtime")] {
        assert_eq!(
            call("fail", &[Val::S64(kind), string("no")]),
            Val::Result(Err(Some(Box::new(Val::Variant(
                case.to_owned(),
                Some(Box::new(string("no")))
            ))))),
            "{case}"
        );
    }
}

#[test]
fn the_ledger_crash_export_traps() {
    let err = try_call("crash", &[]).expect_err("`crash` returned");
    assert!(
        err.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::UnreachableCodeReached),
        "{err:#}"
    );
}
