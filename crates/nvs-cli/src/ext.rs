//! `nvs ext` — `rule:packaging/nvs-ext-is-the-authoring-tool`: the one tool an extension author
//! needs beside their own language's compiler, as six subcommands over one project layout.
//!
//! **What exists.** The command and its six subcommands parse, and each one is dispatched here.
//! A subcommand whose work is not built yet prints that to standard error and exits non-zero,
//! so a script that calls it fails rather than reading an empty success. A project's `nvsx.toml`
//! is read into its manifest by [`nvsx_toml`], whose module doc is the file's reference.
//!
//! **`nvs ext build`** reads `nvsx.toml`, the module it names, the top-level `.wit` files of its
//! `wit` folder and every `.nvs` file under its `source` folder, each list sorted by its relative
//! `/` path so the same project packs to the same bytes. `nvs_ext::pack` makes the `.nvsx`, and
//! the loader boot uses (`crate::extensions::loader`) runs every load check on it under an entry
//! pinning its own digest. Only a file that loads is written, and the command prints its path and
//! the `sha256` line an `[[extension]]` entry pins it with. The packer and the loader own what
//! they check, a WASI preview 1 module among them.
//!
//! **`nvs ext` never writes an `nvs.toml`.** It is not a project command in `initializes`'s
//! sense: `nvs ext test` runs with no configuration unless `--config` names one, so an extension
//! is tested with no grant, and the other subcommands read a project or a `.nvsx` and no
//! configuration at all.
//!
//! **Calls ADR 0246 leaves, settled here under AGENTS.md's priority ordering:**
//!
//! - `build` and `test` take the project directory as an optional argument and default to the
//!   working directory, the way `cargo` does, so the common case is the bare command.
//! - `inspect`, `verify` and `pin` take a `.nvsx` path and never a project, because each answers a
//!   question about a file that may have come from anywhere.
//! - `build` writes the `.nvsx` beside `nvsx.toml`, named for the last segment of the class in
//!   kebab case (`Shop\GeoTools` is `geo-tools.nvsx`). One fixed place per project is what
//!   `nvs ext test` finds without a flag, and the name is the one a configuration's `path` reads.
//!   A toolchain's own output folder is not used, because two toolchains name it differently.
//! - Files under `wit/deps/` are not the author's package: the `nvs:ext` world and its WASI are
//!   compiled into the binary, so `build` reads only the `wit` folder's top level.

mod nvsx_toml;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_ext::load::{Entry, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::pack::{Inputs, pack};
use nvs_ext::source::SourceFile;

use crate::ExtCommand;

/// Runs one `nvs ext` subcommand.
pub(crate) fn run(command: ExtCommand) -> ExitCode {
    match command {
        ExtCommand::New { .. } => unbuilt("new"),
        ExtCommand::Build { project } => answer(build(&project)),
        ExtCommand::Inspect { .. } => unbuilt("inspect"),
        ExtCommand::Test { .. } => unbuilt("test"),
        ExtCommand::Verify { .. } => unbuilt("verify"),
        ExtCommand::Pin { .. } => unbuilt("pin"),
    }
}

/// The exit of a subcommand that wrote `result`'s error, if any, as one line on standard error.
fn answer(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// `nvs ext build`: the `.nvsx` the project in `dir` makes, checked, written and pinned.
fn build(dir: &Path) -> Result<(), String> {
    let project = nvsx_toml::read(dir)?;
    let wasm = std::fs::read(&project.module).map_err(|err| {
        format!(
            "{}: the module `nvsx.toml` names cannot be read: {err}",
            project.module.display()
        )
    })?;
    let wit = wit_files(&project.wit)?;
    let wit: Vec<(&str, &str)> = wit
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let files = match &project.source {
        Some(folder) => source_files(folder)?,
        None => Vec::new(),
    };
    let bytes = pack(&Inputs {
        wasm: &wasm,
        wit: &wit,
        manifest: &project.manifest,
        files: &files,
    })
    .map_err(|err| format!("{}: {err}", project.module.display()))?;

    let manifest = Manifest::parse(&project.manifest).map_err(|err| err.0)?;
    let out = dir.join(format!("{}.nvsx", file_stem(&manifest.class)));
    let sha256 = pin(&bytes);
    crate::extensions::loader()?
        .load_bytes(
            &Entry {
                path: out.clone(),
                sha256: sha256.clone(),
                memory: None,
                grants: nvs_config::extension::Granted::default(),
            },
            &bytes,
        )
        .map_err(|refused| refused.to_string())?;
    std::fs::write(&out, &bytes)
        .map_err(|err| format!("{}: cannot be written: {err}", out.display()))?;
    println!("{}", out.display());
    println!("sha256 = \"{sha256}\"");
    Ok(())
}

/// The file name `class` is written under: its last segment in kebab case.
fn file_stem(class: &str) -> String {
    let last = class.rsplit('\\').next().unwrap_or(class);
    let mut stem = String::with_capacity(last.len() + 4);
    for (i, ch) in last.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            stem.push('-');
        }
        stem.push(ch.to_ascii_lowercase());
    }
    stem
}

/// The `.wit` files at the top level of `folder`, as `(name, text)` sorted by name. A folder that
/// does not exist has none, which is enough for a component.
fn wit_files(folder: &Path) -> Result<Vec<(String, String)>, String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|err| format!("{}: cannot be read: {err}", folder.display()))?
            .path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "wit") {
            let text = read_text(&path)?;
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            files.push((name, text));
        }
    }
    files.sort();
    Ok(files)
}

/// Every `.nvs` file under `folder`, its path relative to `folder` and joined with `/`, sorted.
fn source_files(folder: &Path) -> Result<Vec<SourceFile>, String> {
    let mut files = Vec::new();
    let mut pending: Vec<(PathBuf, String)> = vec![(folder.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|err| format!("{}: cannot be read: {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("{}: cannot be read: {err}", dir.display()))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                pending.push((path, relative));
            } else if path.extension().is_some_and(|ext| ext == "nvs") {
                files.push(SourceFile {
                    path: relative,
                    text: read_text(&path)?,
                });
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The text of the file at `path`, or an error naming it.
fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|err| format!("{}: cannot be read: {err}", path.display()))
}

/// The answer of a subcommand this binary does not implement yet: one line on standard error and
/// a failing exit, so nothing mistakes it for a success.
fn unbuilt(subcommand: &str) -> ExitCode {
    eprintln!("error: `nvs ext {subcommand}` is not available in this version of `nvs`.");
    ExitCode::FAILURE
}
