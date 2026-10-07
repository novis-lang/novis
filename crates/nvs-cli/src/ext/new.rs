//! `nvs ext new`: a project that builds the `Example\Greeting` extension, in Rust or C.
//!
//! The templates live under `crates/nvs-cli/templates/ext/` and are compiled into the binary, so
//! `new` reads nothing but the directory it writes. Each project carries the author's WIT package
//! (`wit/greeting.wit`, whose world includes `nvs:ext/extension`), the `nvs:ext` world and its WASI
//! under `wit/deps/` written from [`nvs_ext::pack::world_wit`] — the copy this binary's packer and
//! loader use — an `nvsx.toml`, the code, a README and one Novis test. `wit/deps/` is the layout
//! `wit-bindgen` reads, and the folder `nvs ext build` skips as not the author's own.
//!
//! The Rust `Cargo.toml` carries its own `[workspace]` table, so a project made inside another
//! Cargo workspace (this repository among them) builds alone. The C project names its two tools,
//! `wit-bindgen-cli` and wasi-sdk, in its README: `nvs` embeds no C bindings generator.
//!
//! A directory that exists and holds anything is refused before a byte is written, so `new` never
//! mixes a template into somebody's files.

use std::path::Path;

use crate::ExtLang;

/// The files every project has, whatever its language, as `(path, text)`.
const COMMON: &[(&str, &str)] = &[
    (
        "wit/greeting.wit",
        include_str!("../../templates/ext/common/wit/greeting.wit"),
    ),
    (
        "tests/GreetingTest.nvs",
        include_str!("../../templates/ext/common/tests/GreetingTest.nvs"),
    ),
];

/// The Rust project's own files.
const RUST: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../../templates/ext/rust/Cargo.toml"),
    ),
    (
        "src/lib.rs",
        include_str!("../../templates/ext/rust/src/lib.rs"),
    ),
    (
        "nvsx.toml",
        include_str!("../../templates/ext/rust/nvsx.toml"),
    ),
    (
        "README.md",
        include_str!("../../templates/ext/rust/README.md"),
    ),
    (".gitignore", "/target/\n/*.nvsx\n"),
];

/// The C project's own files.
const C: &[(&str, &str)] = &[
    (
        "greeting.c",
        include_str!("../../templates/ext/c/greeting.c"),
    ),
    ("nvsx.toml", include_str!("../../templates/ext/c/nvsx.toml")),
    ("README.md", include_str!("../../templates/ext/c/README.md")),
    (".gitignore", "/gen/\n/*.wasm\n/*.nvsx\n"),
];

/// `nvs ext new --lang <lang> <dir>`: the project written into `dir`, which is made if missing.
pub(super) fn new(lang: ExtLang, dir: &Path) -> Result<(), String> {
    if let Ok(mut entries) = std::fs::read_dir(dir)
        && entries.next().is_some()
    {
        return Err(format!(
            "{}: the folder is not empty. `nvs ext new` needs a new or empty folder.",
            dir.display()
        ));
    }
    if dir.exists() && !dir.is_dir() {
        return Err(format!("{}: this is a file, not a folder.", dir.display()));
    }
    let own = match lang {
        ExtLang::Rust => RUST,
        ExtLang::C => C,
    };
    let world = nvs_ext::pack::world_wit().map(|(path, text)| (format!("wit/{path}"), text));
    let files = COMMON
        .iter()
        .chain(own)
        .map(|(path, text)| ((*path).to_owned(), *text))
        .chain(world);
    for (path, text) in files {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("{}: cannot be made: {err}", parent.display()))?;
        }
        std::fs::write(&path, text)
            .map_err(|err| format!("{}: cannot be written: {err}", path.display()))?;
    }
    println!("{}", dir.display());
    Ok(())
}
