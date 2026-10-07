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
//!   "enums": [{"name": "Unit", "cases": ["Metres", "NauticalMiles"], "help": "A unit of distance."}],
//!   "unions": [{"name": "Area", "cases": [
//!     {"name": "Circle", "shape": "{radius: float}"},
//!     {"name": "Box", "shape": "{width: float, height: float}"}]}],
//!   "resources": [{"name": "Route"}],
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
//! tighten (`rule:security/extension-manifest-only-tightens`). Any other qualifier key is an unknown
//! key; a `secret` return, by key or in a type, and a `secret` or `tainted` written into a type are
//! refused by name, so no manifest spelling admits `secret` or launders `tainted`.
//!
//! **`enums`, `unions` and `resources` declare the named types** a signature writes by their short
//! name, and [`Manifest::novis_type`] resolves them. A declared name is not a built-in type's, and
//! a resource is one the interface exports under its name in kebab-case, which the loader checks.
//! An enum's
//! cases are distinct in kebab-case, which is how they cross. A union's case is a shape that may
//! name an enum or a `Core` value class, never another union, and its cases must be told apart by
//! their keys alone: no array is a value of two of them, which holds exactly when, for every two
//! cases, some required field of one is not a field of the other. A value of the union is then the
//! one case whose fields hold all its keys and whose required fields it has.

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use crate::types::{Field, NovisType};
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
    /// The enums the signatures name.
    #[serde(default)]
    pub enums: Vec<Enum>,
    /// The closed unions of shapes the signatures name.
    #[serde(default)]
    pub unions: Vec<Union>,
    /// The resources the signatures name.
    #[serde(default)]
    pub resources: Vec<Resource>,
    /// The `[ext.<name>]` settings block the extension reads, if any.
    #[serde(default)]
    pub settings: Option<Settings>,
    /// The I/O the extension asks for. It holds none of it unless the entry grants it too.
    #[serde(default)]
    pub requests: Requests,
    /// The largest linear memory the extension needs, in bytes.
    #[serde(default)]
    pub memory: Option<u64>,
    /// The files of the same `.nvsx`'s `nvs.source` section. The manifest's JSON never writes
    /// them: [`crate::load::read_manifests`] fills them, so a compiler holds each class beside the
    /// Novis source under its namespace.
    #[serde(skip)]
    pub source: Vec<crate::source::SourceFile>,
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
    /// A `secret` return, which is read only to be refused: nothing `secret` crosses an
    /// extension boundary (`rule:security/secret-does-not-cross-an-extension`).
    #[serde(default)]
    pub secret: bool,
    /// The help text `nvs` shows for the method.
    #[serde(default)]
    pub help: Option<String>,
}

fn void() -> String {
    "void".to_string()
}

/// A key that is present, whatever its value: serde reads a `null` into an `Option` as `None`.
fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

impl Method {
    /// The name of the WIT function this method calls: its name in kebab-case, so `distanceKm` is
    /// `distance-km`.
    pub fn export_name(&self) -> String {
        crate::kebab(&self.name)
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
    /// The value it has when the caller leaves it out. `"default": null` is a default of `null`,
    /// and only a missing key is no default.
    #[serde(default, deserialize_with = "present")]
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

/// An enum a signature names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enum {
    /// The enum's short name.
    pub name: String,
    /// Its cases, in order.
    pub cases: Vec<String>,
    /// The help text `nvs` shows for the enum.
    #[serde(default)]
    pub help: Option<String>,
}

/// A closed union of shapes a signature names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Union {
    /// The union's short name.
    pub name: String,
    /// Its cases, in order.
    pub cases: Vec<UnionCase>,
}

/// One case of a [`Union`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnionCase {
    /// The case's name, which is its WIT case in kebab-case.
    pub name: String,
    /// The case's shape, as Novis text.
    pub shape: String,
}

/// A resource a signature names, which the extension's interface exports.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    /// The resource's short name, which is its WIT name in kebab-case.
    pub name: String,
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

impl Settings {
    /// Checks the `[ext.<name>]` block a configuration writes against the keys declared here,
    /// as boot does before a request runs (ADR 0246 § 9).
    ///
    /// # Errors
    ///
    /// The refusal's message, naming the block and the key, for a key the manifest does not
    /// declare and for a value that is not of its key's type.
    pub fn check(&self, block: &serde_json::Map<String, Value>) -> Result<(), String> {
        for (key, value) in block {
            let Some(declared) = self.keys.iter().find(|k| &k.name == key) else {
                let known: Vec<String> =
                    self.keys.iter().map(|k| format!("`{}`", k.name)).collect();
                let known = if known.is_empty() {
                    "no keys".to_owned()
                } else {
                    known.join(", ")
                };
                return Err(format!(
                    "`[ext.{}]` has an unknown key `{key}`: the extension declares {known}",
                    self.name
                ));
            };
            if !declared.ty.admits(value) {
                return Err(format!(
                    "`[ext.{}]` sets `{key}` to {value}, which is not of its type `{}`",
                    self.name,
                    declared.ty.name()
                ));
            }
        }
        Ok(())
    }
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
    /// The type as a manifest writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
            Self::String => "string",
            Self::Strings => "array<string>",
        }
    }

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

    /// The type `text` writes, resolving the enums, unions and resources this manifest declares,
    /// or why it is outside the table.
    pub fn novis_type(&self, text: &str) -> Result<NovisType, String> {
        NovisType::parse_with(text, &|name| self.named(name, true))
    }

    /// The declared type `name`, where `unions` says whether a union may be named here: a union's
    /// own shapes may not name one.
    fn named(&self, name: &str, unions: bool) -> Option<NovisType> {
        if let Some(decl) = self.enums.iter().find(|decl| decl.name == name) {
            return Some(NovisType::Enum {
                name: decl.name.clone(),
                cases: decl.cases.clone(),
            });
        }
        if let Some(decl) = self.resources.iter().find(|decl| decl.name == name) {
            return Some(NovisType::Resource(decl.name.clone()));
        }
        let decl = self
            .unions
            .iter()
            .find(|decl| unions && decl.name == name)?;
        let cases = self.union_cases(decl).ok()?;
        Some(NovisType::Union {
            name: decl.name.clone(),
            cases,
        })
    }

    /// Each case of `decl` with the fields of its shape.
    fn union_cases(&self, decl: &Union) -> Result<Vec<(String, Vec<Field>)>, Malformed> {
        decl.cases
            .iter()
            .map(|case| {
                NovisType::parse_shape(&case.shape, &|name| self.named(name, false))
                    .map(|fields| (case.name.clone(), fields))
                    .map_err(|err| {
                        malformed(format!(
                            "the case `{}` of `{}` does not read: {err}",
                            case.name, decl.name
                        ))
                    })
            })
            .collect()
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
            let returns = format!("the return of `{}`", method.name);
            if method.secret {
                return Err(secret_crosses(&returns));
            }
            unqualified(&returns, &method.returns)?;
            for param in &method.params {
                let what = format!("the parameter `{}` of `{}`", param.name, method.name);
                unqualified(&what, &param.ty)?;
            }
        }
        unique("constant", self.consts.iter().map(|c| c.name.as_str()))?;
        let names = self.enums.iter().map(|decl| decl.name.as_str());
        let names = names
            .chain(self.unions.iter().map(|decl| decl.name.as_str()))
            .chain(self.resources.iter().map(|decl| decl.name.as_str()));
        unique("type", names.clone())?;
        if let Some(name) = names.into_iter().find(|name| BUILT_IN.contains(name)) {
            return Err(malformed(format!(
                "the type `{name}` has the name of a built-in type"
            )));
        }
        for decl in &self.enums {
            distinct_cases(&decl.name, decl.cases.iter().map(String::as_str))?;
        }
        for decl in &self.unions {
            distinct_cases(&decl.name, decl.cases.iter().map(|case| case.name.as_str()))?;
            let cases = self.union_cases(decl)?;
            for (at, (first, a)) in cases.iter().enumerate() {
                for (second, b) in &cases[at + 1..] {
                    if !apart(a, b) {
                        return Err(malformed(format!(
                            "the cases `{first}` and `{second}` of `{}` cannot be told apart by their keys",
                            decl.name
                        )));
                    }
                }
            }
        }
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

/// Refuses a qualifier written into the type `text` of `what`. `sink` and `source` are the only
/// qualifier declarations, so a `tainted` type has one spelling, and a `secret` one has none.
fn unqualified(what: &str, text: &str) -> Result<(), Malformed> {
    match text.split_whitespace().next() {
        Some("secret") => Err(secret_crosses(what)),
        Some("tainted") => Err(malformed(format!(
            "{what} writes `tainted` in its type: a manifest declares a tainted return with \
             `\"source\": true` and a parameter that refuses one with `\"sink\": true`"
        ))),
        _ => Ok(()),
    }
}

fn secret_crosses(what: &str) -> Malformed {
    malformed(format!(
        "{what} is declared `secret`, and a `secret` value never crosses an extension boundary"
    ))
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

/// The names a declared type may not have.
const BUILT_IN: &[&str] = &[
    "bool", "int", "uint", "float", "string", "bytes", "mixed", "array", "void",
];

/// Refuses an enum or union `of` with no case, or with two cases of one name in kebab-case.
fn distinct_cases<'a>(
    of: &str,
    names: impl Iterator<Item = &'a str> + Clone,
) -> Result<(), Malformed> {
    unique(&format!("case of `{of}`"), names.clone())?;
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(crate::kebab(name)) {
            return Err(malformed(format!(
                "the case `{name}` of `{of}` is another case's name in kebab-case"
            )));
        }
    }
    if seen.is_empty() {
        return Err(malformed(format!("the type `{of}` has no case")));
    }
    Ok(())
}

/// Whether no array is a value of both the shape `a` and the shape `b`: some required field of
/// one is not a field of the other.
fn apart(a: &[Field], b: &[Field]) -> bool {
    let missing = |from: &[Field], into: &[Field]| {
        from.iter()
            .any(|(name, optional, _)| !optional && !into.iter().any(|(n, ..)| n == name))
    };
    missing(a, b) || missing(b, a)
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
