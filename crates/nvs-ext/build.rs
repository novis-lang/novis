//! Builds the built-in components and packs each into the `.nvsx` that `src/builtin.rs` embeds
//! (`rule:packaging/the-first-party-components-are-built-in`), and links integration tests the way
//! every crate's do (`tools/build/tests_without_pdb.rs`).
//!
//! **The packer is compiled into this script with `#[path]`.** A build script cannot call its own
//! crate, and `pack` reaches only `manifest`, `section`, `source`, `types` and the crate-root items
//! in `src/common.rs`, so those six files are this script's modules too, and `nvs ext build` and
//! the build run the same code.
//!
//! Each component's crate under `extensions/` is its own workspace. It is built with `cargo build
//! --release --target wasm32-wasip2 --locked`, run from the crate's directory so its
//! `.cargo/config.toml` applies, into `target/ext/`. The child cargo gets none of the variables
//! that make this build differ from a plain one — `RUSTFLAGS`, a rustc wrapper such as
//! clippy-driver, a coverage build's flags, the parent's target directory — so the component's
//! bytes are the same whichever command built the tree.
//!
//! A component's Novis source is every `.nvs` file directly in its crate's `nvs/` folder, packed
//! into `nvs.source` in the order of their names, each under its file name.
//!
//! Only a build with the `engine` feature embeds the components; a reader of manifests alone
//! builds none.

#![allow(
    clippy::print_stdout,
    reason = "`cargo:` directives on stdout are how a build script talks to Cargo"
)]

#[path = "../../tools/build/tests_without_pdb.rs"]
mod tests_without_pdb;

#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/common.rs"]
mod common;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/manifest.rs"]
mod manifest;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/pack.rs"]
mod pack;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/section.rs"]
mod section;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/source.rs"]
mod source;
#[allow(
    dead_code,
    unreachable_pub,
    reason = "the library's modules, of which `pack` uses part"
)]
#[path = "src/types.rs"]
mod types;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Malformed, is_identifier, kebab, malformed};
use sha2::{Digest, Sha256};

/// The variables a child cargo would read as a change to how it builds, so it never sees them.
const PARENT_BUILD_VARS: &[&str] = &[
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_ENCODED_RUSTDOCFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_TARGET",
    "CARGO_INCREMENTAL",
    "CARGO_LLVM_COV",
    "CARGO_LLVM_COV_TARGET_DIR",
    "LLVM_PROFILE_FILE",
];

/// One built-in component: its crate's directory under `extensions/`, the wasm file its build
/// writes, the name its `.nvsx` and digest are written under in `OUT_DIR`, its WIT file under
/// `wit/`, and the inputs in its directory beside the ones every component has. Every input is a
/// path that exists, because Cargo reruns a script on every build while one it watches does not,
/// and a component's `nvs/` folder is one of them.
struct Component {
    dir: &'static str,
    wasm: &'static str,
    name: &'static str,
    wit: &'static str,
    extra_inputs: &'static [&'static str],
}

const COMPONENTS: &[Component] = &[
    Component {
        dir: "image",
        wasm: "nvs_image.wasm",
        name: "image",
        wit: "wit/image.wit",
        extra_inputs: &["build.rs", "libwebp"],
    },
    Component {
        dir: "intl",
        wasm: "nvs_intl.wasm",
        name: "intl",
        wit: "wit/intl.wit",
        extra_inputs: &[],
    },
];

fn main() {
    tests_without_pdb::main();
    if env::var_os("CARGO_FEATURE_ENGINE").is_none() {
        return;
    }
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let root = manifest_dir.join("../..");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets it"));
    let target = root.join("target/ext");
    for component in COMPONENTS {
        let dir = root.join("extensions").join(component.dir);
        for input in [
            "src",
            "Cargo.toml",
            "Cargo.lock",
            ".cargo",
            "manifest.json",
            "nvs",
        ]
        .iter()
        .chain(component.extra_inputs)
        {
            println!("cargo:rerun-if-changed={}", dir.join(input).display());
        }
        for wit in ["wit/nvs-ext", component.wit] {
            println!("cargo:rerun-if-changed={}", root.join(wit).display());
        }
        let wasm = build(&dir, &target).join(component.wasm);
        let nvsx = pack_component(&dir, &wasm).unwrap_or_else(|err| {
            panic!(
                "the built-in component `{}` does not pack: {err}",
                component.name
            )
        });
        write(&out.join(format!("{}.nvsx", component.name)), &nvsx);
        write(
            &out.join(format!("{}.sha256", component.name)),
            &Sha256::digest(&nvsx),
        );
    }
}

/// Builds the component crate in `dir` into `target`, and returns the directory its wasm is in.
fn build(dir: &Path, target: &Path) -> PathBuf {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(dir)
        .args([
            "build",
            "--release",
            "--locked",
            "--target",
            "wasm32-wasip2",
            "--target-dir",
        ])
        .arg(target);
    for var in PARENT_BUILD_VARS {
        command.env_remove(var);
    }
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("cargo does not start for {}: {err}", dir.display()));
    assert!(
        status.success(),
        "the built-in component in {} does not build for wasm32-wasip2 ({status}); \
         `rustup target add wasm32-wasip2` if the target is missing",
        dir.display()
    );
    target.join("wasm32-wasip2/release")
}

/// The `.nvsx` of the component in `dir`: its built `wasm`, its `manifest.json` and the Novis
/// source in its `nvs/` folder, packed as `nvs ext build` packs a component.
fn pack_component(dir: &Path, wasm: &Path) -> Result<Vec<u8>, Malformed> {
    let read = |path: &Path| {
        fs::read(path).map_err(|err| malformed(format!("{} does not read: {err}", path.display())))
    };
    let wasm = read(wasm)?;
    let manifest = read(&dir.join("manifest.json"))?;
    pack::pack(&pack::Inputs {
        wasm: &wasm,
        wit: &[],
        manifest: &manifest,
        files: &novis_source(&dir.join("nvs"))?,
    })
}

/// Every `.nvs` file directly in `folder`, in the order of their names; none when `folder` does
/// not exist.
fn novis_source(folder: &Path) -> Result<Vec<source::SourceFile>, Malformed> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|err| malformed(format!("{} does not list: {err}", folder.display())))?
            .path();
        if path.extension().is_none_or(|ext| ext != "nvs") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|err| malformed(format!("{} does not read: {err}", path.display())))?;
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        files.push(source::SourceFile {
            path: name.into_owned(),
            text,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap_or_else(|err| panic!("{} is not written: {err}", path.display()));
}
