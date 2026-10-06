//! The two custom sections of a `.nvsx`, found at the component's top level.
//!
//! `wasmparser` walks the binary, because the component format is an external specification and
//! is not ours to parse by hand. Nothing here validates the component: that is the engine's check,
//! made at load. A custom section inside a nested module or component is not the extension's, so
//! only the outermost component's count.

use wasmparser::{Encoding, Parser, Payload};

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
pub fn read(bytes: &[u8]) -> Result<Sections<'_>, Malformed> {
    let mut sections = Sections::default();
    // `parse_all` descends into nested modules and components; each opens with a `Version` and
    // closes with an `End`, so the depth says whose section a payload is.
    let mut depth = 0usize;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload =
            payload.map_err(|err| malformed(format!("the file is not WebAssembly: {err}")))?;
        match payload {
            Payload::Version { encoding, .. } => {
                if depth == 0 && encoding == Encoding::Module {
                    return Err(malformed(
                        "the file is a WebAssembly module, not a component",
                    ));
                }
                depth += 1;
            }
            Payload::End(_) => depth = depth.saturating_sub(1),
            Payload::CustomSection(custom) if depth == 1 => {
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
    Ok(sections)
}
