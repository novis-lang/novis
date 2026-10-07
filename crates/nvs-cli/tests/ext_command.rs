//! `nvs ext`, driven through the built binary: the six subcommands
//! `rule:packaging/nvs-ext-is-the-authoring-tool` names, what each one answers, and that none of
//! them writes an `nvs.toml` into the directory it runs in.

use std::path::Path;
use std::process::Command;

/// The subcommands, in the rule's order.
const SUBCOMMANDS: [&str; 6] = ["new", "build", "inspect", "test", "verify", "pin"];

/// `nvs <args...>` run in `dir`, as `(stdout, stderr, exit code)`.
fn nvs_in(dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// `nvs ext --help` lists every subcommand, and `nvs --help` lists `ext`.
#[test]
fn ext_help_lists_the_six_subcommands() {
    let dir = nvs_repo::scratch("ext-command-help");
    let (out, err, code) = nvs_in(&dir, &["ext", "--help"]);
    assert_eq!(code, Some(0), "`nvs ext --help` succeeds: {err}");
    for subcommand in SUBCOMMANDS {
        assert!(
            out.lines()
                .any(|line| line.trim_start().starts_with(&format!("{subcommand} "))),
            "`{subcommand}` is listed: {out}"
        );
    }
    let (out, err, code) = nvs_in(&dir, &["--help"]);
    assert_eq!(code, Some(0), "`nvs --help` succeeds: {err}");
    assert!(
        out.lines()
            .any(|line| line.trim_start().starts_with("ext ")),
        "`ext` is listed: {out}"
    );
}

/// A subcommand this binary does not implement says so and fails, and writes nothing into the
/// directory it runs in — no `nvs.toml` above all.
#[test]
fn an_unbuilt_subcommand_says_so_and_exits_non_zero() {
    let dir = nvs_repo::scratch("ext-command-unbuilt");
    let calls: [(&str, &[&str]); 5] = [
        ("new", &["ext", "new", "--lang", "rust", "project"]),
        ("inspect", &["ext", "inspect", "geo.nvsx"]),
        ("test", &["ext", "test"]),
        ("verify", &["ext", "verify", "geo.nvsx"]),
        ("pin", &["ext", "pin", "geo.nvsx"]),
    ];
    for (subcommand, args) in calls {
        let (out, err, code) = nvs_in(&dir, args);
        assert_eq!(code, Some(1), "`nvs ext {subcommand}` fails: {err}");
        assert!(out.is_empty(), "nothing goes to standard output: {out}");
        assert!(
            err.contains(&format!("`nvs ext {subcommand}` is not available")),
            "the error names the subcommand: {err}"
        );
    }
    let left: Vec<_> = std::fs::read_dir(&*dir)
        .expect("the scratch directory is readable")
        .map(|entry| entry.expect("an entry is readable").file_name())
        .collect();
    assert!(left.is_empty(), "`nvs ext` wrote nothing here: {left:?}");
}

/// `--lang` takes `rust` and `c` and nothing else, and is required.
#[test]
fn ext_new_takes_rust_or_c() {
    let dir = nvs_repo::scratch("ext-command-lang");
    let (_, err, code) = nvs_in(&dir, &["ext", "new", "--lang", "zig", "project"]);
    assert_eq!(code, Some(2), "an unknown language is a usage error: {err}");
    assert!(
        err.contains("rust") && err.contains('c'),
        "the error lists the two languages: {err}"
    );
    let (_, err, code) = nvs_in(&dir, &["ext", "new", "project"]);
    assert_eq!(code, Some(2), "`--lang` is required: {err}");
}

/// The author's WIT: the interface the manifest names.
const GEO_WIT: &str = r"package shop:geo;

interface api {
    use nvs:ext/types@1.0.0.{error};

    distance-km: func(%from: string, round: bool) -> result<f64, error>;
}
";

/// A project's `nvsx.toml` whose one method matches [`GEO_WIT`].
const NVSX_TOML: &str = r#"class = "Shop\\Geo"
interface = "shop:geo/api"
module = "geo.wasm"
source = "nvs"

[[methods]]
signature = "distanceKm(string $from, bool $round): float"
help = "The distance to a place, in kilometres."
"#;

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

/// The Novis source the project carries, as `(path under nvs/, text)`.
const SOURCE: [(&str, &str); 2] = [
    (
        "Geo/Units.nvs",
        "namespace Shop\\Geo;\n\nconst KM_PER_MILE = 1.609344;\n",
    ),
    ("Format.nvs", "namespace Shop\\Geo;\n"),
];

/// A project in a fresh scratch directory: `nvsx.toml` as `toml`, `wasm` as its module, the WIT
/// and the source files above, and a `README.md` in the source folder that is not packed.
fn project(name: &str, toml: &str, wasm: &[u8]) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(name);
    let write = |path: &str, bytes: &[u8]| {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the folder is made");
        std::fs::write(&path, bytes).expect("the file is written");
    };
    write("nvsx.toml", toml.as_bytes());
    write("geo.wasm", wasm);
    write("wit/geo.wit", GEO_WIT.as_bytes());
    for (path, text) in SOURCE {
        write(&format!("nvs/{path}"), text.as_bytes());
    }
    write("nvs/README.md", b"Not Novis source.\n");
    dir
}

/// The `.nvsx` the project in `dir` wrote, after a build that must succeed.
fn built(dir: &Path) -> Vec<u8> {
    let (out, err, code) = nvs_in(dir, &["ext", "build"]);
    assert_eq!(code, Some(0), "`nvs ext build` succeeds: {err}");
    assert!(out.contains("geo.nvsx"), "{out}");
    std::fs::read(dir.join("geo.nvsx")).expect("the `.nvsx` is written")
}

/// `bytes`, loaded the way an `[[extension]]` entry pinning them loads them.
fn loads(bytes: &[u8]) -> nvs_ext::load::Extension {
    nvs_ext::load::Loader::new(&wasmtime::Engine::default())
        .load_bytes(
            &nvs_ext::load::Entry {
                path: "geo.nvsx".into(),
                sha256: nvs_ext::load::pin(bytes),
                memory: None,
                grants: nvs_config::extension::Granted::default(),
            },
            bytes,
        )
        .expect("the built file loads")
}

/// A build that fails says why on standard error, prints nothing else and writes no `.nvsx`.
fn refused(dir: &Path) -> String {
    let (out, err, code) = nvs_in(dir, &["ext", "build"]);
    assert_eq!(code, Some(1), "`nvs ext build` fails: {err}");
    assert!(out.is_empty(), "nothing goes to standard output: {out}");
    assert!(
        !dir.join("geo.nvsx").exists(),
        "a build that fails writes no `.nvsx`"
    );
    err
}

#[test]
fn nvs_ext_build_packs_a_component_its_manifest_and_its_source_into_one_file() {
    let dir = project("ext-build-component", NVSX_TOML, &component());
    let bytes = built(&dir);
    let extension = loads(&bytes);
    assert_eq!(extension.manifest.class, "Shop\\Geo");
    assert_eq!(
        extension.manifest.methods[0].help.as_deref(),
        Some("The distance to a place, in kilometres.")
    );
    let files: Vec<_> = extension
        .source
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.text.as_str()))
        .collect();
    assert_eq!(files, [SOURCE[1], SOURCE[0]], "every `.nvs` file, sorted");
}

#[test]
fn nvs_ext_build_componentizes_a_core_module() {
    let dir = project("ext-build-core", NVSX_TOML, &core_module());
    let bytes = built(&dir);
    assert_eq!(
        &bytes[4..8],
        [0x0d, 0, 1, 0],
        "a component's version and layer"
    );
    assert_eq!(loads(&bytes).manifest.interface, "shop:geo/api");
    assert_eq!(bytes, built(&dir), "the same project builds the same bytes");
}

#[test]
fn nvs_ext_build_refuses_an_nvsx_toml_key_it_does_not_know_naming_the_line() {
    let toml = NVSX_TOML.replacen("source = \"nvs\"", "colour = \"blue\"", 1);
    let dir = project("ext-build-unknown-key", &toml, &component());
    let err = refused(&dir);
    assert!(err.contains("nvsx.toml:4: "), "the line is named: {err}");
    assert!(err.contains("colour"), "the key is named: {err}");
}

#[test]
fn nvs_ext_build_refuses_a_signature_the_exports_do_not_match() {
    let toml = NVSX_TOML.replacen(
        "distanceKm(string $from, bool $round)",
        "distanceKm(string $from)",
        1,
    );
    let dir = project("ext-build-mismatch", &toml, &component());
    let err = refused(&dir);
    assert!(err.contains("does not load"), "{err}");
    assert!(err.contains("distance-km"), "the export is named: {err}");
}

#[test]
fn nvs_ext_build_refuses_a_preview_1_module_naming_wasm32_wasip2() {
    let module = wat::parse_str(
        r#"(module
  (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
  (func (export "shop:geo/api#distance-km") (param i32 i32 i32) (result i32) i32.const 16))"#,
    )
    .expect("the test module compiles");
    let dir = project("ext-build-preview-1", NVSX_TOML, &module);
    let err = refused(&dir);
    assert!(err.contains("wasi_snapshot_preview1"), "{err}");
    assert!(
        err.contains("`wasm32-wasip2`"),
        "the target is named: {err}"
    );
}

#[test]
fn nvs_ext_build_prints_the_path_and_the_pin_of_what_it_wrote() {
    let dir = project("ext-build-prints", NVSX_TOML, &component());
    let (out, err, code) = nvs_in(&dir, &["ext", "build"]);
    assert_eq!(code, Some(0), "`nvs ext build` succeeds: {err}");
    let bytes = std::fs::read(dir.join("geo.nvsx")).expect("the `.nvsx` is written");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    assert!(Path::new(lines[0]).ends_with("geo.nvsx"), "the path: {out}");
    assert_eq!(
        lines[1],
        format!("sha256 = \"{}\"", nvs_ext::load::pin(&bytes))
    );

    let elsewhere = nvs_repo::scratch("ext-build-elsewhere");
    let (out, err, code) = nvs_in(&elsewhere, &["ext", "build", &dir.display().to_string()]);
    assert_eq!(code, Some(0), "a project named by path builds: {err}");
    assert_eq!(
        Path::new(out.lines().next().unwrap_or_default()),
        dir.join("geo.nvsx")
    );
    let left = std::fs::read_dir(&*elsewhere)
        .expect("the scratch directory is readable")
        .count();
    assert_eq!(left, 0, "nothing is written where the command runs");
}
