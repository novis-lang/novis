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
// covers: tools:cli/nvs-ext-new-and-nvs-ext-build
#[test]
fn nvs_ext_help_lists_all_six_commands() {
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

/// `nvs ext new --lang <lang> greeting` run in a fresh scratch directory, which must succeed and
/// print the project's path; the directory is returned.
fn new_project(name: &str, lang: &str) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(name);
    let (out, err, code) = nvs_in(&dir, &["ext", "new", "--lang", lang, "greeting"]);
    assert_eq!(code, Some(0), "`nvs ext new --lang {lang}` succeeds: {err}");
    assert_eq!(out.trim(), "greeting", "the project's path is printed");
    assert!(
        !dir.join("nvs.toml").exists() && !dir.join("greeting/nvs.toml").exists(),
        "`nvs ext new` writes no `nvs.toml`"
    );
    dir
}

/// The text of `path` under the project `dir/greeting`.
fn written(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join("greeting").join(path))
        .unwrap_or_else(|err| format!("`{path}` is not written: {err}"))
}

/// Every file of the world under `wit/nvs-ext/` is in the project under `wit/deps/`, byte for
/// byte, each WASI package as one file and the `nvs:ext` package as a folder of its own.
fn assert_the_binarys_own_world(dir: &Path) {
    let world = nvs_repo::path("wit/nvs-ext");
    let mut compared = 0;
    for entry in std::fs::read_dir(&world).expect("the world is on disk") {
        let path = entry.expect("an entry is readable").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            for dep in std::fs::read_dir(&path).expect("the WASI folder is on disk") {
                let dep = dep.expect("an entry is readable").path();
                let name = dep
                    .file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned();
                let want = std::fs::read_to_string(&dep).expect("a WASI file reads");
                assert_eq!(written(dir, &format!("wit/deps/{name}")), want, "{name}");
                compared += 1;
            }
        } else {
            let want = std::fs::read_to_string(&path).expect("a world file reads");
            assert_eq!(
                written(dir, &format!("wit/deps/nvs-ext/{name}")),
                want,
                "{name}"
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "the world has files to compare");
    let package = written(dir, "wit/greeting.wit");
    assert!(
        package.contains("include nvs:ext/extension@1.0.0;") && package.contains("export api;"),
        "the author's world includes the extension world: {package}"
    );
}

// covers: tools:cli/nvs-ext-new-and-nvs-ext-build
#[test]
fn nvs_ext_new_writes_a_rust_project_with_the_binarys_own_world() {
    let dir = new_project("ext-new-rust", "rust");
    assert_the_binarys_own_world(&dir);
    let cargo = written(&dir, "Cargo.toml");
    assert!(
        cargo.lines().any(|line| line.trim() == "[workspace]"),
        "the project is a workspace of its own: {cargo}"
    );
    assert!(cargo.contains("crate-type = [\"cdylib\"]"), "{cargo}");
    assert!(written(&dir, "src/lib.rs").contains("world: \"greeting\""));
    let toml = written(&dir, "nvsx.toml");
    assert!(
        toml.contains("module = \"target/wasm32-wasip2/release/greeting.wasm\""),
        "{toml}"
    );
    assert!(written(&dir, "tests/GreetingTest.nvs").contains("#[Test]"));
}

#[test]
fn nvs_ext_new_writes_a_c_project_with_the_binarys_own_world() {
    let dir = new_project("ext-new-c", "c");
    assert_the_binarys_own_world(&dir);
    let readme = written(&dir, "README.md");
    assert!(
        readme.contains("wit-bindgen-cli") && readme.contains("wasi-sdk"),
        "the README names both tools: {readme}"
    );
    assert!(written(&dir, "greeting.c").contains("#include \"greeting.h\""));
    assert!(written(&dir, "nvsx.toml").contains("module = \"greeting.wasm\""));
    assert!(written(&dir, "tests/GreetingTest.nvs").contains("#[Test]"));
    assert!(
        !dir.join("greeting/Cargo.toml").exists(),
        "the C project has no `Cargo.toml`"
    );
}

/// A folder that already holds a file is refused, and nothing is written into it.
#[test]
fn nvs_ext_new_refuses_a_folder_that_is_not_empty() {
    let dir = nvs_repo::scratch("ext-new-not-empty");
    std::fs::create_dir(dir.join("greeting")).expect("the folder is made");
    std::fs::write(dir.join("greeting/notes.txt"), b"mine\n").expect("the file is written");
    let (out, err, code) = nvs_in(&dir, &["ext", "new", "--lang", "rust", "greeting"]);
    assert_eq!(code, Some(1), "`nvs ext new` fails: {err}");
    assert!(out.is_empty(), "nothing goes to standard output: {out}");
    assert!(err.contains("not empty"), "the error says why: {err}");
    let left: Vec<_> = std::fs::read_dir(dir.join("greeting"))
        .expect("the folder is readable")
        .map(|entry| entry.expect("an entry is readable").file_name())
        .collect();
    assert_eq!(left, ["notes.txt"], "nothing was added");
    assert_eq!(
        written(&dir, "notes.txt"),
        "mine\n",
        "the file is untouched"
    );
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

// covers: tools:cli/nvs-ext-new-and-nvs-ext-build
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

// covers: tools:cli/nvs-ext-new-and-nvs-ext-build
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

// covers: tools:cli/nvs-ext-new-and-nvs-ext-build
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

/// [`NVSX_TOML`] with a request for files and for one host.
fn requesting() -> String {
    format!("{NVSX_TOML}\n[requests]\nread = [\"data/geo/\"]\nconnect = [\"tiles.example.com\"]\n")
}

/// The names of the files in `dir`, sorted.
fn listed(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("the directory is readable")
        .map(|entry| {
            entry
                .expect("an entry is readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

// covers: tools:cli/nvs-ext-inspect-nvs-ext-verify-and-nvs-ext-pin
#[test]
fn nvs_ext_inspect_prints_the_manifest_and_the_io_it_requests() {
    let dir = project("ext-inspect", &requesting(), &component());
    let bytes = built(&dir);
    let before = listed(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "inspect", "geo.nvsx"]);
    assert_eq!(code, Some(0), "`nvs ext inspect` succeeds: {err}");
    for expected in [
        "class      Shop\\Geo",
        "interface  shop:geo/api",
        "world      nvs:ext@1.0.0",
        &format!("sha256     {}", nvs_ext::load::pin(&bytes)),
        "  read     data/geo/",
        "  connect  tiles.example.com",
        "  distanceKm(string $from, bool $round): float",
        "      The distance to a place, in kilometres.",
        "  Format.nvs",
        "  Geo/Units.nvs",
    ] {
        assert!(
            out.lines().any(|line| line == expected),
            "`{expected}` is printed: {out}"
        );
    }
    assert!(
        !out.contains("KM_PER_MILE"),
        "the source text is printed only with `--source`: {out}"
    );
    assert_eq!(listed(&dir), before, "`inspect` writes nothing");

    let bare = project("ext-inspect-bare", NVSX_TOML, &component());
    built(&bare);
    let (out, err, code) = nvs_in(&bare, &["ext", "inspect", "geo.nvsx"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        out.contains("requests\n  none\n"),
        "a file that requests nothing says so: {out}"
    );
}

#[test]
fn nvs_ext_inspect_source_prints_every_source_file_under_its_path() {
    let dir = project("ext-inspect-source", NVSX_TOML, &component());
    built(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "inspect", "--source", "geo.nvsx"]);
    assert_eq!(code, Some(0), "`nvs ext inspect --source` succeeds: {err}");
    for (path, text) in SOURCE {
        assert!(
            out.contains(&format!("\n--- {path}\n{text}")),
            "`{path}` is printed under its path: {out}"
        );
    }
    assert!(
        !out.contains("Not Novis source."),
        "a file that was not packed is not printed: {out}"
    );
}

#[test]
fn nvs_ext_inspect_escapes_control_characters_from_the_file() {
    let toml = NVSX_TOML.replacen("in kilometres.", "in \\u001b[2Jkilometres.\\u202e", 1);
    let dir = project("ext-inspect-escapes", &toml, &component());
    std::fs::write(
        dir.join("nvs/Format.nvs"),
        "namespace Shop\\Geo;\n// \u{1b}]0;title\u{7}\rover\n",
    )
    .expect("the file is written");
    built(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "inspect", "--source", "geo.nvsx"]);
    assert_eq!(code, Some(0), "{err}");
    for raw in ['\u{1b}', '\u{7}', '\r', '\u{202e}'] {
        assert!(
            !out.contains(raw),
            "{raw:?} from the file reaches the terminal: {out:?}"
        );
    }
    assert!(out.contains("in \\u{1b}[2Jkilometres.\\u{202e}"), "{out}");
    assert!(out.contains("// \\u{1b}]0;title\\u{7}\\u{d}over"), "{out}");
}

#[test]
fn nvs_ext_verify_passes_a_file_boot_would_load() {
    let dir = project("ext-verify-passes", NVSX_TOML, &component());
    built(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "verify", "geo.nvsx"]);
    assert_eq!(code, Some(0), "`nvs ext verify` succeeds: {err}");
    assert_eq!(out, "geo.nvsx: passes every check loading it runs\n");
}

/// A manifest for [`component`], with `world` and `class` as given.
fn manifest(world: &str, class: &str, params: &str) -> String {
    format!(
        r#"{{"manifest": 1, "world": "{world}", "class": "{class}", "interface": "shop:geo/api",
"methods": [{{"name": "distanceKm", "params": [{params}], "returns": "float"}}]}}"#
    )
}

/// `wasm`, packed by the packer `nvs ext build` uses with `manifest` and no source.
fn packed(wasm: &[u8], manifest: &str) -> Vec<u8> {
    nvs_ext::pack::pack(&nvs_ext::pack::Inputs {
        wasm,
        wit: &[("geo.wit", GEO_WIT)],
        manifest: manifest.as_bytes(),
        files: &[],
    })
    .expect("the packer takes it")
}

// covers: tools:cli/nvs-ext-inspect-nvs-ext-verify-and-nvs-ext-pin
#[test]
fn nvs_ext_verify_names_every_refusal_boot_would_make() {
    const PARAMS: &str = r#"{"name": "from", "type": "string"}, {"name": "round", "type": "bool"}"#;
    let outside = wat::parse_str(
        r#"(component
  (import "wasi:sockets/tcp@0.2.9" (instance))
  (instance $api)
  (export "shop:geo/api" (instance $api)))"#,
    )
    .expect("the test component compiles");
    let cases: [(&str, Vec<u8>, &str); 6] = [
        (
            "not-wasm.nvsx",
            b"not wasm".to_vec(),
            "not a valid component",
        ),
        ("bare.nvsx", component(), "no `nvs.manifest` section"),
        (
            "newer.nvsx",
            packed(&component(), &manifest("2.0.0", "Shop\\\\Geo", PARAMS)),
            "nvs:ext@2.0.0",
        ),
        (
            "outside.nvsx",
            packed(&outside, &manifest("1.0.0", "Shop\\\\Geo", "")),
            "`wasi:sockets/tcp@0.2.9`",
        ),
        (
            "reserved.nvsx",
            packed(&component(), &manifest("1.0.0", "Novis\\\\Geo", PARAMS)),
            "is under `Novis\\`",
        ),
        (
            "mismatch.nvsx",
            packed(
                &component(),
                &manifest(
                    "1.0.0",
                    "Shop\\\\Geo",
                    r#"{"name": "from", "type": "string"}"#,
                ),
            ),
            "shop:geo/api#distance-km",
        ),
    ];
    let dir = nvs_repo::scratch("ext-verify-refusals");
    for (file, bytes, reason) in cases {
        std::fs::write(dir.join(file), bytes).expect("the file is written");
        let (out, err, code) = nvs_in(&dir, &["ext", "verify", file]);
        assert_eq!(code, Some(1), "`{file}` fails: {err}");
        assert!(out.is_empty(), "nothing goes to standard output: {out}");
        assert!(err.contains(file), "the file is named: {err}");
        assert!(err.contains(reason), "`{file}`: `{reason}` is named: {err}");
    }
}

#[test]
fn nvs_ext_verify_instantiates_nothing() {
    // A core module whose start function traps: instantiating the component would fail.
    let trapping = wat::parse_str(
        r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (core module $m
    (memory (export "memory") 1)
    (func $trap unreachable)
    (start $trap)
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
    .expect("the test component compiles");
    let dir = project("ext-verify-instantiates-nothing", NVSX_TOML, &trapping);
    built(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "verify", "geo.nvsx"]);
    assert_eq!(code, Some(0), "no guest code ran: {err}");
    assert!(out.contains("passes"), "{out}");
}

// covers: tools:cli/nvs-ext-inspect-nvs-ext-verify-and-nvs-ext-pin
#[test]
fn nvs_ext_pin_prints_an_entry_nvs_config_check_accepts() {
    let dir = project("ext-pin", NVSX_TOML, &component());
    let bytes = built(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "pin", "geo.nvsx"]);
    assert_eq!(code, Some(0), "`nvs ext pin` succeeds: {err}");
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    assert_eq!(lines[0], "[[extension]]");
    assert_eq!(
        lines[2],
        format!("sha256 = \"{}\"", nvs_ext::load::pin(&bytes))
    );

    // Pasted into a configuration somewhere else, the entry still names the file.
    let config = nvs_repo::scratch("ext-pin-config");
    std::fs::write(config.join("nvs.toml"), &out).expect("the configuration is written");
    let (_, err, code) = nvs_in(&config, &["config", "check"]);
    assert_eq!(code, Some(0), "`nvs config check` accepts the entry: {err}");
    let table: toml::Table = toml::from_str(&out).expect("the entry is TOML");
    let entry = &table["extension"].as_array().expect("an array of tables")[0];
    let path = entry["path"].as_str().expect("`path` is a string");
    assert!(Path::new(path).is_absolute(), "{path}");
    nvs_ext::load::Loader::new(&wasmtime::Engine::default())
        .load(&nvs_ext::load::Entry {
            path: path.into(),
            sha256: entry["sha256"]
                .as_str()
                .expect("`sha256` is a string")
                .into(),
            memory: None,
            grants: nvs_config::extension::Granted::default(),
        })
        .expect("the entry loads its file");

    std::fs::write(dir.join("broken.nvsx"), b"not wasm").expect("the file is written");
    let (out, err, code) = nvs_in(&dir, &["ext", "pin", "broken.nvsx"]);
    assert_eq!(
        code,
        Some(1),
        "a file that does not load is not pinned: {err}"
    );
    assert!(out.is_empty(), "no entry is printed: {out}");
}

// covers: tools:cli/nvs-ext-inspect-nvs-ext-verify-and-nvs-ext-pin
#[test]
fn nvs_ext_pin_never_writes_a_grant() {
    let dir = project("ext-pin-no-grant", &requesting(), &component());
    built(&dir);
    let before = listed(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "pin", "geo.nvsx"]);
    assert_eq!(code, Some(0), "{err}");
    for requested in ["grants", "data/geo/", "tiles.example.com"] {
        assert!(!out.contains(requested), "`{requested}` is printed: {out}");
    }
    assert_eq!(
        listed(&dir),
        before,
        "`pin` writes nothing, `nvs.toml` least of all"
    );
}

/// A test file calling the extension's one method, which the component above answers with `0.0`,
/// and expecting `expected`.
fn geo_test(expected: &str) -> String {
    format!(
        "<?nvs
use Core\\Test;
use Shop\\Geo;

final class GeoTest {{
    #[Test]
    public function measuresADistance(): void {{
        Test::assertSame(Geo::distanceKm(\"a\", true), {expected});
    }}
}}
"
    )
}

/// A built project whose `tests` folder holds [`geo_test`] expecting `expected`.
fn tested_project(name: &str, expected: &str) -> nvs_repo::Scratch {
    let dir = project(name, NVSX_TOML, &component());
    built(&dir);
    std::fs::create_dir_all(dir.join("tests")).expect("the folder is made");
    std::fs::write(dir.join("tests/GeoTest.nvs"), geo_test(expected)).expect("the test is written");
    dir
}

/// Sets the modification time of the file at `path` to `seconds` after `file`'s.
fn touch_after(path: &Path, file: &Path, seconds: u64) {
    let then = std::fs::metadata(file)
        .and_then(|meta| meta.modified())
        .expect("the file has a modification time")
        + std::time::Duration::from_secs(seconds);
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|opened| opened.set_modified(then))
        .expect("the modification time is set");
}

// covers: tools:cli/nvs-ext-test
#[test]
fn nvs_ext_test_runs_the_projects_tests_with_the_built_file_loaded() {
    let dir = tested_project("ext-test-runs", "0.0");
    let (out, err, code) = nvs_in(&dir, &["ext", "test"]);
    assert_eq!(code, Some(0), "the test passes: {out}{err}");
    assert!(out.contains("0 failed, 1 passed"), "{out}{err}");

    // The project directory is an argument, and a failing test fails the command.
    std::fs::write(dir.join("tests/GeoTest.nvs"), geo_test("1.5")).expect("the test is written");
    let parent = dir.parent().expect("the scratch folder has a parent");
    let name = dir
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();
    let (out, err, code) = nvs_in(parent, &["ext", "test", &name]);
    assert_eq!(code, Some(1), "the test fails: {out}{err}");
    assert!(out.contains("1 failed, 0 passed"), "{out}{err}");
}

// covers: tools:cli/nvs-ext-test
#[test]
fn nvs_ext_test_reads_no_nvs_toml_and_grants_nothing() {
    let dir = tested_project("ext-test-no-config", "0.0");
    let before = listed(&dir);
    let (out, err, code) = nvs_in(&dir, &["ext", "test"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(listed(&dir), before, "`test` writes no `nvs.toml`");

    // An `nvs.toml` beside the project that would refuse any run that read it.
    std::fs::write(
        dir.join("nvs.toml"),
        "[[extension]]\npath = 'geo.nvsx'\nsha256 = \"00\"\ngrants = { read = ['.'] }\n",
    )
    .expect("the configuration is written");
    let (out, err, code) = nvs_in(&dir, &["ext", "test"]);
    assert_eq!(code, Some(0), "`./nvs.toml` is not read: {out}{err}");
    assert!(out.contains("0 failed, 1 passed"), "{out}{err}");
}

// covers: tools:cli/nvs-ext-test
#[test]
fn nvs_ext_test_refuses_a_built_file_older_than_its_module() {
    let dir = tested_project("ext-test-stale", "0.0");
    touch_after(&dir.join("geo.wasm"), &dir.join("geo.nvsx"), 60);
    let (out, err, code) = nvs_in(&dir, &["ext", "test"]);
    assert_eq!(code, Some(1), "a stale build is refused: {out}{err}");
    assert!(!out.contains("passed"), "no test ran: {out}");
    assert!(err.contains("geo.wasm"), "the newer input is named: {err}");
    assert!(err.contains("nvs ext build"), "the fix is named: {err}");

    // A source file the build packs counts as an input too.
    let dir = tested_project("ext-test-stale-source", "0.0");
    touch_after(&dir.join("nvs/Format.nvs"), &dir.join("geo.nvsx"), 60);
    let (_, err, code) = nvs_in(&dir, &["ext", "test"]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("Format.nvs"), "{err}");
}

#[test]
fn nvs_ext_test_refuses_a_config_whose_entry_pin_differs() {
    let dir = tested_project("ext-test-pin", "0.0");
    let wrong = "0".repeat(64);
    std::fs::write(
        dir.join("ci.toml"),
        format!("[[extension]]\npath = 'geo.nvsx'\nsha256 = \"{wrong}\"\n"),
    )
    .expect("the configuration is written");
    let (out, err, code) = nvs_in(&dir, &["ext", "test", "--config", "ci.toml"]);
    assert_eq!(code, Some(1), "a differing pin is refused: {out}{err}");
    assert!(!out.contains("passed"), "no test ran: {out}");
    assert!(err.contains(&wrong), "the configured pin is named: {err}");
    let pin = nvs_ext::load::pin(&std::fs::read(dir.join("geo.nvsx")).expect("the file"));
    assert!(err.contains(&pin), "the built file's pin is named: {err}");

    // The same entry with the right pin runs the tests.
    std::fs::write(
        dir.join("ci.toml"),
        format!("[[extension]]\npath = 'geo.nvsx'\nsha256 = \"{pin}\"\n"),
    )
    .expect("the configuration is written");
    let (out, err, code) = nvs_in(&dir, &["ext", "test", "--config", "ci.toml"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(out.contains("0 failed, 1 passed"), "{out}{err}");
}
