//! The `nvs.source` section: the Novis source files that build on the extension's class.
//!
//! The payload is JSON, `{"source": 1, "files": [{"path": "Geo/Units.nvs", "text": "…"}]}`. A path
//! is relative to the extension's own source root: `/`-separated, with no empty, `.` or `..`
//! segment, no `\` and no `:`, and it names a `.nvs` file. Two files with one path are refused.
//! Whether each file declares a namespace under the extension's own is the compiler's check, which
//! reads the text.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Malformed, malformed};

/// The source format this host reads.
pub const FORMAT: u64 = 1;

/// The source files an extension carries, in the order the section lists them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The format version, always [`FORMAT`] once parsed.
    pub source: u64,
    /// The files.
    #[serde(default)]
    pub files: Vec<SourceFile>,
}

/// One source file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    /// The file's path, relative to the extension's source root.
    pub path: String,
    /// The file's contents.
    pub text: String,
}

#[derive(Deserialize)]
struct Head {
    source: Option<Value>,
}

impl Source {
    /// The source files the section payload `bytes` holds.
    pub fn parse(bytes: &[u8]) -> Result<Self, Malformed> {
        let head: Head = serde_json::from_slice(bytes)
            .map_err(|err| malformed(format!("the source section is not a JSON object: {err}")))?;
        match head.source {
            Some(Value::Number(n)) if n.as_u64() == Some(FORMAT) => {}
            Some(other) => {
                return Err(malformed(format!(
                    "the source section has format {other}, and this version of Novis reads format {FORMAT}"
                )));
            }
            None => {
                return Err(malformed(
                    "the source section has no `source` format number",
                ));
            }
        }
        let source: Self = serde_json::from_slice(bytes)
            .map_err(|err| malformed(format!("the source section does not read: {err}")))?;
        let mut seen = HashSet::new();
        for file in &source.files {
            if !is_relative_source_path(&file.path) {
                return Err(malformed(format!(
                    "the source path `{}` is not a relative path to a `.nvs` file",
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
        Ok(source)
    }
}

/// Whether `path` stays inside the source root and names a `.nvs` file.
pub fn is_relative_source_path(path: &str) -> bool {
    path.ends_with(".nvs")
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}
