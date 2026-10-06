//! The `nvs.manifest` section: the class an extension declares, and everything the compiler and the
//! loader need to know about it without running it.
//!
//! The shape is ADR 0246 § 3's list, as JSON:
//!
//! ```json
//! {
//!   "manifest": 1,
//!   "world": "1.0.0",
//!   "class": "Shop\\Geo",
//!   "interface": "shop:geo/api",
//!   "methods": [{
//!     "name": "distanceKm",
//!     "params": [{"name": "from", "type": "string", "sink": true},
//!                {"name": "round", "type": "bool", "default": false}],
//!     "returns": "float",
//!     "source": false,
//!     "help": "The distance between two places, in kilometres."
//!   }],
//!   "consts": [{"name": "EARTH_RADIUS_KM", "type": "float", "value": 6371.0}],
//!   "settings": {"name": "geo", "keys": [{"name": "precision", "type": "int", "default": 6}]},
//!   "requests": {"read": ["data/geo/"], "connect": ["tiles.example.com"]},
//!   "memory": 67108864
//! }
//! ```
//!
//! Types are kept as the Novis text the author wrote; whether each one is in
//! `rule:packaging/a-value-crosses-as-its-wit-type`'s table, and whether it matches the export, is
//! the loader's check against the component. What this module refuses is a manifest that does not
//! read: a format other than `1`, an unknown key, a name that is not an identifier, a name used
//! twice, and a setting whose default is not of its type. `sink` and `source` are the only
//! qualifier declarations (`rule:security/extension-declares-sink-or-source`), and both only
//! tighten.

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use crate::{Malformed, is_identifier, malformed};

/// The manifest format this host reads.
pub const FORMAT: u64 = 1;

/// One extension's manifest.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The format version, always [`FORMAT`] once parsed.
    pub manifest: u64,
    /// The `nvs:ext` world version the component was built against.
    pub world: WorldVersion,
    /// The one class the extension declares, namespace-qualified with `\`.
    pub class: String,
    /// The exported WIT interface the class maps, as `namespace:package/interface`.
    pub interface: String,
    /// The class's methods, one per export.
    #[serde(default)]
    pub methods: Vec<Method>,
    /// The class's `const` members.
    #[serde(default)]
    pub consts: Vec<Const>,
    /// The `[ext.<name>]` settings block the extension reads, if any.
    #[serde(default)]
    pub settings: Option<Settings>,
    /// The I/O the extension asks for. It holds none of it unless the entry grants it too.
    #[serde(default)]
    pub requests: Requests,
    /// The largest linear memory the extension needs, in bytes.
    #[serde(default)]
    pub memory: Option<u64>,
}

/// A `major.minor.patch` version of the `nvs:ext` world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct WorldVersion {
    /// The major version. A host loads only its own.
    pub major: u64,
    /// The minor version. A host loads its own and every older one.
    pub minor: u64,
    /// The patch version, which loading ignores.
    pub patch: u64,
}

impl TryFrom<String> for WorldVersion {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let parts: Vec<&str> = text.split('.').collect();
        let number = |part: &str| {
            (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
                .then(|| part.parse::<u64>().ok())
                .flatten()
        };
        match parts.as_slice() {
            [major, minor, patch] => match (number(major), number(minor), number(patch)) {
                (Some(major), Some(minor), Some(patch)) => Ok(Self {
                    major,
                    minor,
                    patch,
                }),
                _ => Err(format!("`{text}` is not a version like `1.0.0`")),
            },
            _ => Err(format!("`{text}` is not a version like `1.0.0`")),
        }
    }
}

impl fmt::Display for WorldVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One method of the class, and the export it calls.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    /// The method's Novis name.
    pub name: String,
    /// Its parameters, in order. A trailing options shape is the last one.
    #[serde(default)]
    pub params: Vec<Param>,
    /// Its Novis return type.
    #[serde(default = "void")]
    pub returns: String,
    /// The return is always `tainted`: the extension is a source.
    #[serde(default)]
    pub source: bool,
    /// The help text `nvs` shows for the method.
    #[serde(default)]
    pub help: Option<String>,
}

fn void() -> String {
    "void".to_string()
}

impl Method {
    /// The name of the WIT function this method calls: its name in kebab-case, so `distanceKm` is
    /// `distance-km`.
    pub fn export_name(&self) -> String {
        let mut out = String::with_capacity(self.name.len() + 4);
        for (i, c) in self.name.chars().enumerate() {
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
}

/// One parameter of a method.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    /// The parameter's name, which a named argument uses.
    pub name: String,
    /// Its Novis type.
    #[serde(rename = "type")]
    pub ty: String,
    /// The value it has when the caller leaves it out.
    #[serde(default)]
    pub default: Option<Value>,
    /// A `tainted` argument is refused here: the extension is a sink for it.
    #[serde(default)]
    pub sink: bool,
}

/// One `const` member of the class.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Const {
    /// The constant's name.
    pub name: String,
    /// Its Novis type.
    #[serde(rename = "type")]
    pub ty: String,
    /// Its value.
    pub value: Value,
}

/// The `[ext.<name>]` block the extension reads through `nvs:ext/settings`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The block's name, the `<name>` of `[ext.<name>]`.
    pub name: String,
    /// The keys the block may hold.
    #[serde(default)]
    pub keys: Vec<SettingKey>,
}

/// One key of the settings block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingKey {
    /// The key's name.
    pub name: String,
    /// The key's type.
    #[serde(rename = "type")]
    pub ty: SettingType,
    /// The value the guest reads when the block does not set the key.
    pub default: Value,
}

/// The types a setting may have (ADR 0246 § 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SettingType {
    /// `bool`.
    #[serde(rename = "bool")]
    Bool,
    /// `int`.
    #[serde(rename = "int")]
    Int,
    /// `uint`.
    #[serde(rename = "uint")]
    Uint,
    /// `float`.
    #[serde(rename = "float")]
    Float,
    /// `string`.
    #[serde(rename = "string")]
    String,
    /// `array<string>`, a list of strings.
    #[serde(rename = "array<string>")]
    Strings,
}

impl SettingType {
    /// Whether `value` is of this type.
    fn admits(self, value: &Value) -> bool {
        match self {
            Self::Bool => value.is_boolean(),
            Self::Int => value.is_i64(),
            Self::Uint => value.is_u64(),
            Self::Float => value.is_number(),
            Self::String => value.is_string(),
            Self::Strings => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
        }
    }
}

/// The I/O a manifest requests. Each list is one of the three grant kinds
/// (`rule:security/extension-grants-are-an-intersection`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requests {
    /// Directories it reads.
    #[serde(default)]
    pub read: Vec<String>,
    /// Directories it writes.
    #[serde(default)]
    pub write: Vec<String>,
    /// Hosts it connects to over HTTP.
    #[serde(default)]
    pub connect: Vec<String>,
}

/// Only the format number, read before the rest so a newer format is refused by its number.
#[derive(Deserialize)]
struct Head {
    manifest: Option<Value>,
}

impl Manifest {
    /// The manifest the section payload `bytes` holds.
    pub fn parse(bytes: &[u8]) -> Result<Self, Malformed> {
        let head: Head = serde_json::from_slice(bytes)
            .map_err(|err| malformed(format!("the manifest is not a JSON object: {err}")))?;
        match head.manifest {
            Some(Value::Number(n)) if n.as_u64() == Some(FORMAT) => {}
            Some(other) => {
                return Err(malformed(format!(
                    "the manifest has format {other}, and this version of Novis reads format {FORMAT}"
                )));
            }
            None => return Err(malformed("the manifest has no `manifest` format number")),
        }
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|err| malformed(format!("the manifest does not read: {err}")))?;
        manifest.check()?;
        Ok(manifest)
    }

    /// The checks a well-typed manifest still owes: names, and settings defaults.
    fn check(&self) -> Result<(), Malformed> {
        if !self.class.split('\\').all(is_identifier) {
            return Err(malformed(format!(
                "the class `{}` is not a class name",
                self.class
            )));
        }
        if !is_interface(&self.interface) {
            return Err(malformed(format!(
                "the interface `{}` is not written as `namespace:package/interface`",
                self.interface
            )));
        }
        unique("method", self.methods.iter().map(|m| m.name.as_str()))?;
        for method in &self.methods {
            unique(
                &format!("parameter of `{}`", method.name),
                method.params.iter().map(|p| p.name.as_str()),
            )?;
        }
        unique("constant", self.consts.iter().map(|c| c.name.as_str()))?;
        if let Some(settings) = &self.settings {
            if !is_identifier(&settings.name) {
                return Err(malformed(format!(
                    "the settings block `{}` is not a name",
                    settings.name
                )));
            }
            unique("setting", settings.keys.iter().map(|k| k.name.as_str()))?;
            for key in &settings.keys {
                if !key.ty.admits(&key.default) {
                    return Err(malformed(format!(
                        "the setting `{}` has the default {}, which is not of its type",
                        key.name, key.default
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Refuses a name in `names` that is not an identifier or that appears twice.
fn unique<'a>(what: &str, names: impl Iterator<Item = &'a str>) -> Result<(), Malformed> {
    let mut seen = HashSet::new();
    for name in names {
        if !is_identifier(name) {
            return Err(malformed(format!("the {what} `{name}` is not a name")));
        }
        if !seen.insert(name) {
            return Err(malformed(format!("the {what} `{name}` is declared twice")));
        }
    }
    Ok(())
}

/// Whether `name` is `namespace:package/interface`, each part a WIT identifier: lower-case words
/// joined by `-`.
fn is_interface(name: &str) -> bool {
    let wit = |part: &str| {
        !part.is_empty()
            && part.split('-').all(|word| {
                word.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                    && word
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    };
    let Some((namespace, rest)) = name.split_once(':') else {
        return false;
    };
    let Some((package, interface)) = rest.split_once('/') else {
        return false;
    };
    wit(namespace) && wit(package) && wit(interface)
}
