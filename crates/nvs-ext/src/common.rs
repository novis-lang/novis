//! The items the manifest modules reach through `crate::`, in a file of their own so the build
//! script compiles it beside them. `build.rs` packs the built-in components with [`crate::pack`]
//! and compiles `pack`, `manifest`, `section`, `source` and `types` into itself with `#[path]`,
//! so whatever those five name at the crate root lives here.

use std::fmt;

/// Why a `.nvsx`'s sections or their payloads do not read, or why the packer cannot make one. The
/// message names the part that is wrong, and the loader adds which entry it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed(pub String);

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Malformed {}

/// The error for `message`.
pub(crate) fn malformed(message: impl Into<String>) -> Malformed {
    Malformed(message.into())
}

/// The WIT name of the Novis name `name`: kebab-case, so `distanceKm` and `distance_km` are both
/// `distance-km`.
pub(crate) fn kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('-') {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether `name` is a Novis identifier: a letter or `_`, then letters, digits and `_`.
pub(crate) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
