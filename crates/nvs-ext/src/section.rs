//! The two custom sections of a `.nvsx`, found at the component's top level.
//!
//! `wasmparser` walks the binary, because the component format is an external specification and
//! is not ours to parse by hand. Nothing here validates the component: that is the engine's check,
//! made at load. A custom section inside a nested module or component is not the extension's, so
//! only the outermost component's count.

use wasmparser::{Chunk, Encoding, Parser, Payload};

use crate::{Malformed, malformed};

/// The custom section holding the manifest JSON.
pub const MANIFEST: &str = "nvs.manifest";

/// The custom section holding the Novis source files.
pub const SOURCE: &str = "nvs.source";

/// The payloads of the two sections, borrowed from the file. A section the file does not carry is
/// `None`; whether that is allowed is the loader's call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sections<'a> {
    /// The `nvs.manifest` payload.
    pub manifest: Option<&'a [u8]>,
    /// The `nvs.source` payload.
    pub source: Option<&'a [u8]>,
}

/// The two sections of the component `bytes`. A core module, bytes that are not WebAssembly, and a
/// section written twice are refused.
///
/// Only the outer component is walked. A nested module or component is skipped by its length and
/// never parsed: its sections are not the extension's, and the built-in components nest large
/// ones that every `nvs run` would otherwise read at start.
pub fn read(bytes: &[u8]) -> Result<Sections<'_>, Malformed> {
    let mut sections = Sections::default();
    let mut parser = Parser::new(0);
    let mut rest = bytes;
    loop {
        let (payload, consumed) = match parser
            .parse(rest, true)
            .map_err(|err| malformed(format!("the file is not WebAssembly: {err}")))?
        {
            Chunk::Parsed { payload, consumed } => (payload, consumed),
            Chunk::NeedMoreData(_) => {
                return Err(malformed(
                    "the file is not WebAssembly: it ends inside a section",
                ));
            }
        };
        rest = &rest[consumed..];
        match payload {
            Payload::Version {
                encoding: Encoding::Module,
                ..
            } => {
                return Err(malformed(
                    "the file is a WebAssembly module, not a component",
                ));
            }
            // The outer parser has already counted these bytes, so they are stepped over here.
            Payload::ModuleSection {
                unchecked_range, ..
            }
            | Payload::ComponentSection {
                unchecked_range, ..
            } => {
                let len = usize::try_from(unchecked_range.end - unchecked_range.start)
                    .ok()
                    .filter(|len| *len <= rest.len())
                    .ok_or_else(|| {
                        malformed("the file is not WebAssembly: a nested module runs past its end")
                    })?;
                rest = &rest[len..];
            }
            Payload::End(_) => return Ok(sections),
            Payload::CustomSection(custom) => {
                let slot = match custom.name() {
                    MANIFEST => &mut sections.manifest,
                    SOURCE => &mut sections.source,
                    _ => continue,
                };
                if slot.is_some() {
                    return Err(malformed(format!(
                        "the section `{}` appears twice",
                        custom.name()
                    )));
                }
                *slot = Some(custom.data());
            }
            _ => {}
        }
    }
}
