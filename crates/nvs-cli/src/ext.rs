//! `nvs ext` — `rule:packaging/nvs-ext-is-the-authoring-tool`: the one tool an extension author
//! needs beside their own language's compiler, as six subcommands over one project layout.
//!
//! **What exists.** The command and its six subcommands, each dispatched here. A project's
//! `nvsx.toml` is read into its manifest by [`nvsx_toml`], whose module doc is the file's
//! reference.
//!
//! **`nvs ext new`** writes a Rust or C project from the templates [`new`]'s module doc
//! describes, with the `nvs:ext` world's WIT copied from this binary under `wit/deps/`.
//!
//! **`nvs ext build`** reads `nvsx.toml`, the module it names, the top-level `.wit` files of its
//! `wit` folder and every `.nvs` file under its `source` folder, each list sorted by its relative
//! `/` path so the same project packs to the same bytes. `nvs_ext::pack` makes the `.nvsx`, and
//! the loader boot uses (`crate::extensions::loader`) runs every load check on it under an entry
//! pinning its own digest. Only a file that loads is written, and the command prints its path and
//! the `sha256` line an `[[extension]]` entry pins it with. The packer and the loader own what
//! they check, a WASI preview 1 module among them.
//!
//! **`nvs ext inspect`** reads the file's two sections and compiles nothing, so it answers for a
//! file that does not load too: the class, interface, world and `sha256`, the I/O the manifest
//! requests, each method's signature and help, the consts, the settings block and the source
//! paths, and with `--source` each source file's text under a `--- <path>` line. Every string the
//! file carries is printed with its control characters and its text-reordering Unicode marks
//! escaped as `\u{..}`, so a file read before it is trusted cannot drive the terminal. Errors, on
//! every subcommand, are escaped the same way.
//!
//! **`nvs ext test`** runs the `#[Test]` methods of every `.nvs` file under the project's `tests`
//! folder as one program, the way `nvs test <dir>` does, with the `.nvsx` `build` writes in the
//! extension set. It refuses a file older than any input `build` reads, so a test never runs
//! against a stale build. The tree is `crate::config::extension_test_tree`'s: the shipped
//! defaults, or the files `--config` names, and never `./nvs.toml`. With no `--config` the file
//! is added by an entry pinning its own digest and granting nothing. A configuration that lists
//! the file keeps its entry and its grants, and one whose entry pins another digest is refused,
//! naming both, before anything compiles.
//!
//! **`nvs ext verify`** runs the loader boot uses on the file under an entry pinning its own
//! digest and granting nothing. That loader compiles the component and never instantiates it, so
//! no guest code runs. Of boot's refusals, two depend on the configuration and not on the file:
//! a pin that differs, and a class another entry already declares. Every other one is made here.
//!
//! **`nvs ext pin`** runs the same checks first and prints nothing for a file that fails them. It
//! prints `[[extension]]`, `path` and `sha256`, and never `grants`: what a file may reach is the
//! operator's to write after reading `inspect`. `path` is the file's absolute path, because a
//! relative one resolves against the configuration file it is pasted into, which `pin` cannot know.
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
//! - `pin` refuses a file that does not load rather than printing an entry boot would refuse.
//! - `build` writes the `.nvsx` beside `nvsx.toml`, named for the last segment of the class in
//!   kebab case (`Shop\GeoTools` is `geo-tools.nvsx`). One fixed place per project is what
//!   `nvs ext test` finds without a flag, and the name is the one a configuration's `path` reads.
//!   A toolchain's own output folder is not used, because two toolchains name it differently.
//! - Files under `wit/deps/` are not the author's package: the `nvs:ext` world and its WASI are
//!   compiled into the binary, so `build` reads only the `wit` folder's top level.

mod new;
mod nvsx_toml;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_ext::load::{Entry, Extension, pin};
use nvs_ext::manifest::Manifest;
use nvs_ext::pack::{Inputs, pack};
use nvs_ext::source::SourceFile;

use crate::ExtCommand;

/// Runs one `nvs ext` subcommand. `config` is the run's `--config` list, which only `test` reads.
pub(crate) fn run(command: ExtCommand, config: &[PathBuf]) -> ExitCode {
    match command {
        ExtCommand::New { lang, dir } => answer(new::new(lang, &dir)),
        ExtCommand::Build { project } => answer(build(&project)),
        ExtCommand::Inspect { file, source } => answer(inspect(&file, source)),
        ExtCommand::Test { project } => match test(&project, config) {
            Ok(code) => code,
            Err(err) => answer(Err(err)),
        },
        ExtCommand::Verify { file } => answer(verify(&file)),
        ExtCommand::Pin { file } => answer(pin_entry(&file)),
    }
}

/// The exit of a subcommand that wrote `result`'s error, if any, as one line on standard error.
fn answer(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let err: Vec<String> = err.lines().map(escaped).collect();
            eprintln!("error: {}", err.join("\n"));
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
    let extension = load(&out, &bytes)?;
    std::fs::write(&out, &bytes)
        .map_err(|err| format!("{}: cannot be written: {err}", out.display()))?;
    println!("{}", out.display());
    println!("sha256 = \"{}\"", extension.sha256);
    Ok(())
}

/// `bytes`, the file at `path`, loaded by the loader boot uses under an entry pinning their own
/// digest and granting nothing. Loading compiles the component and never instantiates it.
fn load(path: &Path, bytes: &[u8]) -> Result<Extension, String> {
    crate::extensions::loader()?
        .load_bytes(
            &Entry {
                path: path.to_path_buf(),
                sha256: pin(bytes),
                memory: None,
                grants: nvs_config::extension::Granted::default(),
            },
            bytes,
        )
        .map_err(|refused| refused.to_string())
}

/// `nvs ext test`: the `#[Test]` methods under the project's `tests` folder, run with the built
/// `.nvsx` loaded under its own pin, once the file is checked to be newer than every input
/// `build` read.
fn test(dir: &Path, config: &[PathBuf]) -> Result<ExitCode, String> {
    let project = nvsx_toml::read(dir)?;
    let manifest = Manifest::parse(&project.manifest).map_err(|err| err.0)?;
    let file = dir.join(format!("{}.nvsx", file_stem(&manifest.class)));
    let file = std::path::absolute(&file)
        .map_err(|err| format!("{}: cannot be made absolute: {err}", file.display()))?;
    let bytes = std::fs::read(&file).map_err(|err| {
        format!(
            "{}: cannot be read: {err}. Run `nvs ext build` first.",
            file.display()
        )
    })?;
    fresh(&file, dir, &project)?;
    let tests = dir.join("tests");
    let program = match crate::Program::named_by(&tests) {
        Ok(Some(program)) => program,
        Ok(None) => {
            return Err(format!("{}: holds no `.nvs` test file", tests.display()));
        }
        Err(err) => return Err(err),
    };
    let tree = match crate::config::extension_test_tree(config, &tests, &file, &pin(&bytes)) {
        Ok(tree) => tree,
        Err(code) => return Ok(code),
    };
    Ok(crate::run_program_tests(
        &program,
        &tree,
        crate::runner::Format::Human,
        None,
        crate::runner::Flags {
            update: false,
            list: false,
        },
        &crate::coverage::Requested::default(),
    ))
}

/// Refuses `file` when any input `build` read is newer than it: `nvsx.toml`, the module, the
/// top-level `.wit` files and the source files.
fn fresh(file: &Path, dir: &Path, project: &nvsx_toml::Project) -> Result<(), String> {
    let modified = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .map_err(|err| format!("{}: cannot be read: {err}", path.display()))
    };
    let built = modified(file)?;
    let mut inputs = vec![dir.join("nvsx.toml"), project.module.clone()];
    inputs.extend(
        wit_files(&project.wit)?
            .into_iter()
            .map(|(name, _)| project.wit.join(name)),
    );
    if let Some(folder) = &project.source {
        inputs.extend(
            source_files(folder)?
                .into_iter()
                .map(|source| folder.join(source.path)),
        );
    }
    for input in inputs {
        if modified(&input)? > built {
            return Err(format!(
                "{}: is older than {}. Run `nvs ext build` again.",
                file.display(),
                input.display()
            ));
        }
    }
    Ok(())
}

/// The bytes of the `.nvsx` at `file`, or an error naming it.
fn read_nvsx(file: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(file).map_err(|err| format!("{}: cannot be read: {err}", file.display()))
}

/// `nvs ext verify`: every check boot runs on `file`, and one line saying it passed.
fn verify(file: &Path) -> Result<(), String> {
    load(file, &read_nvsx(file)?)?;
    println!(
        "{}: passes every check loading it runs",
        escaped(&file.display().to_string())
    );
    Ok(())
}

/// `nvs ext pin`: the `[[extension]]` entry for `file`, printed once the file passes every load
/// check. The entry has no `grants`.
fn pin_entry(file: &Path) -> Result<(), String> {
    let extension = load(file, &read_nvsx(file)?)?;
    let path = std::path::absolute(file)
        .map_err(|err| format!("{}: cannot be made absolute: {err}", file.display()))?;
    println!("[[extension]]");
    println!("path   = {}", toml_string(&path.display().to_string()));
    println!("sha256 = \"{}\"", extension.sha256);
    Ok(())
}

/// `text` as a TOML basic string: quoted, with `\`, `"` and every control character escaped.
fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(ch))),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// `nvs ext inspect`: what `file` declares and the I/O it requests, read from its two sections
/// without compiling the component. `source` prints every source file's text as well.
fn inspect(file: &Path, source: bool) -> Result<(), String> {
    let bytes = read_nvsx(file)?;
    let refuse = |reason: String| format!("{}: {reason}", file.display());
    let sections = nvs_ext::section::read(&bytes).map_err(|err| refuse(err.0))?;
    let manifest = sections
        .manifest
        .ok_or_else(|| refuse("the file has no `nvs.manifest` section".to_owned()))?;
    let manifest = Manifest::parse(manifest).map_err(|err| refuse(err.0))?;
    let files = match sections.source {
        Some(section) => {
            nvs_ext::source::Source::parse(section)
                .map_err(|err| refuse(err.0))?
                .files
        }
        None => Vec::new(),
    };
    print!("{}", describe(&manifest, &pin(&bytes), &files, source));
    Ok(())
}

/// The text `nvs ext inspect` prints for `manifest`. Every string the file carries goes through
/// [`escaped`], so a file cannot write a control character to the terminal.
fn describe(manifest: &Manifest, sha256: &str, files: &[SourceFile], source: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let e = escaped;
    let _ = writeln!(out, "class      {}", e(&manifest.class));
    let _ = writeln!(out, "interface  {}", e(&manifest.interface));
    let _ = writeln!(out, "world      nvs:ext@{}", manifest.world);
    let _ = writeln!(out, "sha256     {sha256}");
    if let Some(memory) = manifest.memory {
        let _ = writeln!(out, "memory     {memory} bytes");
    }

    let requests = &manifest.requests;
    let _ = writeln!(out, "\nrequests");
    if requests.read.is_empty() && requests.write.is_empty() && requests.connect.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for (kind, list) in [
        ("read", &requests.read),
        ("write", &requests.write),
        ("connect", &requests.connect),
    ] {
        if !list.is_empty() {
            let list: Vec<String> = list.iter().map(|item| e(item)).collect();
            let _ = writeln!(out, "  {kind:<8} {}", list.join(", "));
        }
    }

    let _ = writeln!(out, "\nmethods");
    for method in &manifest.methods {
        let params: Vec<String> = method
            .params
            .iter()
            .map(|param| match &param.default {
                Some(default) => format!("{} ${} = {default}", param.ty, param.name),
                None => format!("{} ${}", param.ty, param.name),
            })
            .collect();
        let tainted = if method.source { "tainted " } else { "" };
        let signature = format!(
            "{}({}): {tainted}{}",
            method.name,
            params.join(", "),
            method.returns
        );
        let _ = writeln!(out, "  {}", e(&signature));
        let sinks: Vec<String> = method
            .params
            .iter()
            .filter(|param| param.sink)
            .map(|param| format!("${}", param.name))
            .collect();
        if !sinks.is_empty() {
            let _ = writeln!(out, "      refuses tainted {}", e(&sinks.join(", ")));
        }
        if let Some(help) = &method.help {
            let _ = writeln!(out, "      {}", e(help));
        }
    }
    if !manifest.consts.is_empty() {
        let _ = writeln!(out, "\nconsts");
        for constant in &manifest.consts {
            let line = format!("{}: {} = {}", constant.name, constant.ty, constant.value);
            let _ = writeln!(out, "  {}", e(&line));
        }
    }
    if let Some(settings) = &manifest.settings {
        let _ = writeln!(out, "\nsettings   [ext.{}]", e(&settings.name));
        for key in &settings.keys {
            let line = format!("{}: {} = {}", key.name, key.ty.name(), key.default);
            let _ = writeln!(out, "  {}", e(&line));
        }
    }

    let _ = writeln!(out, "\nsource");
    if files.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for file in files {
        let _ = writeln!(out, "  {}", e(&file.path));
    }
    if source {
        for file in files {
            let _ = writeln!(out, "\n--- {}", e(&file.path));
            for line in file.text.lines() {
                let _ = writeln!(out, "{}", e(line));
            }
        }
    }
    out
}

/// `text` with every character that changes how a terminal shows the rest escaped as `\u{..}`:
/// the control characters, line breaks among them, and the Unicode marks that reorder text.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let reorders = matches!(
            ch,
            '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        );
        if (ch.is_control() && ch != '\t') || reorders {
            out.push_str(&format!("\\u{{{:x}}}", u32::from(ch)));
        } else {
            out.push(ch);
        }
    }
    out
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
