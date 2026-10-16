//! The `nvs.source` section: the Novis source files that build on the extension's class.
//!
//! The payload is JSON, `{"source": 1, "files": [{"path": "Geo/Units.nvs", "text": "…"}]}`. A path
//! is relative to the extension's own source root: `/`-separated, with no empty, `.` or `..`
//! segment, no `\` and no `:`, and it names a `.nvs` file. Two files with one path are refused.
//!
//! The extension's namespace is its class's minus the last segment: `Shop\Ledger` owns `Shop\`, as
//! `Novis\Image\Codec` owns `Novis\Image\`. [`Source::outside`] finds a file whose first
//! declaration is not `namespace` with a name under it, and the loader refuses the load naming that
//! file. It reads only the file's head — `<?nvs`, whitespace and comments, then `namespace Name;` —
//! so the loader needs no parser. Whether the rest of the file compiles, and whether its path
//! agrees with its names, is the compiler's check.

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

impl Source {
    /// The first file that does not declare `namespace` or a namespace under it, and the
    /// namespace it declares, or `None` when it declares none. Segments compare in any case, as
    /// a class name does.
    pub fn outside(&self, namespace: &str) -> Option<(&SourceFile, Option<&str>)> {
        let own: Vec<&str> = namespace.split('\\').filter(|s| !s.is_empty()).collect();
        self.files.iter().find_map(|file| {
            let declared = declared_namespace(&file.text);
            let under = declared.is_some_and(|declared| {
                let segments: Vec<&str> = declared.split('\\').collect();
                segments.len() >= own.len()
                    && own
                        .iter()
                        .zip(&segments)
                        .all(|(own, seg)| own.eq_ignore_ascii_case(seg))
            });
            (!under).then_some((file, declared))
        })
    }
}

/// The namespace the file `text` declares: its first token past `<?nvs` and the comments must be
/// `namespace`, followed by a qualified name and `;`.
pub fn declared_namespace(text: &str) -> Option<&str> {
    let mut rest = text.strip_prefix('\u{feff}').unwrap_or(text);
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("<?nvs") {
            rest = after;
        } else if rest.starts_with("//") || (rest.starts_with('#') && !rest.starts_with("#[")) {
            rest = rest.find('\n').map_or("", |end| &rest[end..]);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = &after[after.find("*/")? + 2..];
        } else {
            break;
        }
    }
    let after = rest.strip_prefix("namespace")?;
    if !after.starts_with(char::is_whitespace) {
        return None;
    }
    let name = after[..after.find(';')?].trim();
    name.split('\\')
        .all(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
                && segment.chars().all(|c| c.is_alphanumeric() || c == '_')
        })
        .then_some(name)
}

/// Whether `path` stays inside the source root and names a `.nvs` file.
pub fn is_relative_source_path(path: &str) -> bool {
    path.ends_with(".nvs")
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}
