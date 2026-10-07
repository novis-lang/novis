//! The packer: an author's built wasm, a manifest and source files, made into one `.nvsx`.
//!
//! `nvs ext build` and the binary's own build of the built-in components both call [`pack`]
//! (`rule:packaging/nvs-ext-is-the-authoring-tool`). It reads nothing from the disk and links no
//! engine: whether the result loads is checked by its caller, which holds a `Loader`.
//!
//! - **A component** is taken as it is. One that already carries `nvs.manifest` or `nvs.source` is
//!   refused, because the loader refuses a section written twice.
//! - **A core module** is componentized with `wit-component` against a world the packer writes:
//!   `include nvs:ext/extension` plus `export <the manifest's interface>`. The `nvs:ext` WIT and its
//!   WASI dependencies are compiled into this crate from `wit/nvs-ext/`, so the world is always the
//!   one this host implements, and the manifest must name that version. The author's own WIT
//!   package, which defines the interface, is an input. The module's exports follow the canonical
//!   ABI's names (`shop:geo/api#distance-km`, `cabi_realloc`), as every bindings generator writes
//!   them. A module importing WASI preview 1 (`wasi_snapshot_preview1`) is refused naming
//!   `wasm32-wasip2`, the target that builds against the world's WASI 0.2.
//! - **The manifest** is parsed to check it and then written byte for byte, so `nvs ext inspect`
//!   prints what the author's tool wrote.
//! - **The source files** are checked with [`is_relative_source_path`] and written as the
//!   `nvs.source` JSON in the order given.
//!
//! The two sections are appended at the component's end by [`append_section`], manifest first.
//! Nothing in the output depends on the clock, the machine or a hash order, so the same inputs give
//! the same bytes, and so the same pin.

use std::collections::HashSet;

use serde::Serialize;
use wasmparser::Parser;
use wit_component::{ComponentEncoder, StringEncoding, embed_component_metadata};
use wit_parser::{Resolve, SourceMap, UnresolvedPackageGroup};

use crate::manifest::Manifest;
use crate::section::{self, MANIFEST, SOURCE};
use crate::source::{self, SourceFile, is_relative_source_path};
use crate::{Malformed, malformed};

/// The `nvs:ext` package's WIT files, and the WASI packages it imports from.
const WORLD_WIT: &[(&str, &str)] = &[
    ("world.wit", include_str!("../../../wit/nvs-ext/world.wit")),
    ("types.wit", include_str!("../../../wit/nvs-ext/types.wit")),
    ("log.wit", include_str!("../../../wit/nvs-ext/log.wit")),
    (
        "settings.wit",
        include_str!("../../../wit/nvs-ext/settings.wit"),
    ),
];
const WASI_WIT: &[(&str, &str)] = &[
    (
        "deps/cli.wit",
        include_str!("../../../wit/nvs-ext/deps/cli.wit"),
    ),
    (
        "deps/clocks.wit",
        include_str!("../../../wit/nvs-ext/deps/clocks.wit"),
    ),
    (
        "deps/filesystem.wit",
        include_str!("../../../wit/nvs-ext/deps/filesystem.wit"),
    ),
    (
        "deps/http.wit",
        include_str!("../../../wit/nvs-ext/deps/http.wit"),
    ),
    (
        "deps/io.wit",
        include_str!("../../../wit/nvs-ext/deps/io.wit"),
    ),
    (
        "deps/random.wit",
        include_str!("../../../wit/nvs-ext/deps/random.wit"),
    ),
    (
        "deps/sockets.wit",
        include_str!("../../../wit/nvs-ext/deps/sockets.wit"),
    ),
];

/// What a `.nvsx` is packed from.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    /// The author's built wasm: a component, or a core module.
    pub wasm: &'a [u8],
    /// The author's WIT package, as `(path, text)` files, which defines the manifest's interface.
    /// Only a core module reads it; the path is used in error messages.
    pub wit: &'a [(&'a str, &'a str)],
    /// The manifest JSON.
    pub manifest: &'a [u8],
    /// The Novis source files the extension carries.
    pub files: &'a [SourceFile],
}

#[derive(Serialize)]
struct SourceSection<'a> {
    source: u64,
    files: &'a [SourceFile],
}

/// The `.nvsx` made from `inputs`.
pub fn pack(inputs: &Inputs<'_>) -> Result<Vec<u8>, Malformed> {
    let manifest = Manifest::parse(inputs.manifest)?;
    let mut seen = HashSet::new();
    for file in inputs.files {
        if !is_relative_source_path(&file.path) {
            return Err(malformed(format!(
                "the source path `{}` is not a relative `.nvs` path inside the project",
                file.path
            )));
        }
        if !seen.insert(file.path.as_str()) {
            return Err(malformed(format!(
                "the source path `{}` appears twice",
                file.path
            )));
        }
    }
    let source = serde_json::to_vec(&SourceSection {
        source: source::FORMAT,
        files: inputs.files,
    })
    .map_err(|err| malformed(format!("the source files do not write as JSON: {err}")))?;

    let component = if Parser::is_component(inputs.wasm) {
        let sections = section::read(inputs.wasm)?;
        for (name, present) in [
            (MANIFEST, sections.manifest.is_some()),
            (SOURCE, sections.source.is_some()),
        ] {
            if present {
                return Err(malformed(format!(
                    "the component already carries the section `{name}`"
                )));
            }
        }
        inputs.wasm.to_vec()
    } else if Parser::is_core_wasm(inputs.wasm) {
        if let Some(module) = preview_1_import(inputs.wasm)? {
            return Err(malformed(format!(
                "the module imports `{module}`, so it was built for WASI preview 1; build it for `wasm32-wasip2`"
            )));
        }
        componentize(inputs.wasm, inputs.wit, &manifest)?
    } else {
        return Err(malformed(
            "the file is neither a WebAssembly component nor a core module",
        ));
    };
    let bytes = append_section(component, MANIFEST, inputs.manifest);
    Ok(append_section(bytes, SOURCE, &source))
}

/// The component the core module `module` makes, its world the extension world plus the
/// manifest's interface from the author's `wit`.
fn componentize(
    module: &[u8],
    wit: &[(&str, &str)],
    manifest: &Manifest,
) -> Result<Vec<u8>, Malformed> {
    let mut deps = vec![parse("nvs:ext", WORLD_WIT)?];
    for file in WASI_WIT {
        deps.push(parse("WASI", std::slice::from_ref(file))?);
    }
    if !wit.is_empty() {
        deps.push(parse("the extension's", wit)?);
    }
    let world_text = format!(
        "package nvs:nvsx;\nworld extension {{\n  include nvs:ext/extension@{world};\n  export {interface};\n}}\n",
        world = manifest.world,
        interface = manifest.interface,
    );
    let main = parse("the packer's", &[("nvsx.wit", world_text.as_str())])?;
    let mut resolve = Resolve::new();
    let package = resolve
        .push_groups(main, deps)
        .map_err(|err| malformed(format!("the WIT does not resolve: {err:#}")))?;
    let world = resolve
        .select_world(&[package], Some("extension"))
        .map_err(|err| malformed(format!("the WIT does not resolve: {err:#}")))?;
    let mut module = module.to_vec();
    embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
        .map_err(|err| malformed(format!("the module's type does not encode: {err:#}")))?;
    ComponentEncoder::default()
        .validate(true)
        .module(&module)
        .and_then(|mut encoder| encoder.encode())
        .map_err(|err| {
            malformed(format!(
                "the module does not componentize against the world: {err:#}"
            ))
        })
}

/// The WASI preview 1 module the core module `module` imports from, if it imports from one.
fn preview_1_import(module: &[u8]) -> Result<Option<&str>, Malformed> {
    let unreadable =
        |err: wasmparser::BinaryReaderError| malformed(format!("the module does not read: {err}"));
    for payload in Parser::new(0).parse_all(module) {
        if let wasmparser::Payload::ImportSection(reader) = payload.map_err(unreadable)? {
            for import in reader.into_imports() {
                let import = import.map_err(unreadable)?;
                if matches!(import.module, "wasi_snapshot_preview1" | "wasi_unstable") {
                    return Ok(Some(import.module));
                }
            }
        }
    }
    Ok(None)
}

/// The WIT package `files` make, `whose` naming it in an error.
fn parse(whose: &str, files: &[(&str, &str)]) -> Result<UnresolvedPackageGroup, Malformed> {
    let mut map = SourceMap::new();
    for (path, text) in files {
        map.push_str(path, *text);
    }
    map.parse()
        .map_err(|(_, err)| malformed(format!("{whose} WIT does not parse: {err}")))
}

/// `bytes` with a custom section `name` holding `data` appended. A custom section is valid at any
/// point of a component's top level, so the end is as good as anywhere.
pub fn append_section(mut bytes: Vec<u8>, name: &str, data: &[u8]) -> Vec<u8> {
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
    let mut body = Vec::with_capacity(name.len() + data.len() + 5);
    leb(&mut body, name.len());
    body.extend_from_slice(name.as_bytes());
    body.extend_from_slice(data);
    bytes.push(0);
    leb(&mut bytes, body.len());
    bytes.extend(body);
    bytes
}
