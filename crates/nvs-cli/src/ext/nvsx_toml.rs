//! An extension project's `nvsx.toml`: what an author writes, read into the `nvs.manifest` JSON
//! `nvs_ext::manifest::Manifest::parse` reads (`rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`).
//!
//! This is the only writer of a manifest, and the JSON it writes is parsed by the loader's own
//! parser before [`read`] returns, so a project whose manifest would not load fails here, naming
//! `nvsx.toml`. The file:
//!
//! ```toml
//! class = "Shop\\Geo"
//! interface = "shop:geo/api"
//! module = "target/wasm32-wasip2/release/geo.wasm"
//! wit = "wit"          # the author's WIT package; `wit` when left out
//! source = "nvs"       # the Novis source folder; no source when left out
//! world = "1.0.0"      # the `nvs:ext` version; this binary's own when left out
//! memory = 67108864
//! resources = ["Route"]
//! consts = ["float EARTH_RADIUS_KM = 6371.0"]
//!
//! [[methods]]
//! signature = "distanceKm(string $from, string $to, bool $round = false): float"
//! sink = ["from"]
//! help = "The distance between two places, in kilometres."
//!
//! [[enums]]
//! name = "Unit"
//! cases = ["Metres", "NauticalMiles"]
//!
//! [[unions]]
//! name = "Area"
//! cases = [{ name = "Circle", shape = "{radius: float}" }]
//!
//! [settings]
//! name = "geo"
//! keys = [{ name = "precision", type = "int", default = 6 }]
//!
//! [requests]
//! read = ["data/geo/"]
//! connect = ["tiles.example.com"]
//! ```
//!
//! **A signature and a constant are Novis text**, so an author writes the language a program calls
//! them from: `function` and `public static` may lead a signature, a parameter is `type $name` with
//! an optional `= literal`, and a missing return type is `void`. A return type written `tainted T`
//! is a source (`rule:security/extension-declares-sink-or-source`). A sink has no Novis spelling,
//! because a Novis parameter has no way to say it refuses `tainted` while its twins propagate it, so
//! it is the method's `sink` list of parameter names. A default is a literal: `null`, a bool, a
//! number, a quoted string or a list of literals in `[...]`.
//!
//! Every other key mirrors the manifest's own, and an unknown key is refused with the file and the
//! line, the way `nvs.toml` refuses one. A signature or a constant that does not read names the line
//! it is on.

use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Number, Value, json};
use toml::Spanned;

/// The file a project's manifest is written in.
pub(crate) const FILE: &str = "nvsx.toml";

/// A project's `nvsx.toml`, read and checked.
#[derive(Debug)]
pub(crate) struct Project {
    /// The author's built wasm, a module or a component.
    pub(crate) module: PathBuf,
    /// The folder of the author's WIT package.
    pub(crate) wit: PathBuf,
    /// The folder of the Novis source, if the extension carries any.
    pub(crate) source: Option<PathBuf>,
    /// The `nvs.manifest` JSON.
    pub(crate) manifest: Vec<u8>,
}

/// The `nvsx.toml` in `dir`, with each path in it resolved against `dir`.
pub(crate) fn read(dir: &Path) -> Result<Project, String> {
    let path = dir.join(FILE);
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("{}: cannot be read: {err}", path.display()))?;
    let parsed = parse(&path.display().to_string(), &text)?;
    Ok(Project {
        module: dir.join(parsed.module),
        wit: dir.join(parsed.wit),
        source: parsed.source.map(|folder| dir.join(folder)),
        manifest: parsed.manifest,
    })
}

/// What [`parse`] reads, with its paths as the file writes them.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub(crate) module: String,
    pub(crate) wit: String,
    pub(crate) source: Option<String>,
    pub(crate) manifest: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Toml {
    class: String,
    interface: String,
    module: String,
    #[serde(default = "default_wit")]
    wit: String,
    source: Option<String>,
    world: Option<String>,
    memory: Option<u64>,
    #[serde(default)]
    methods: Vec<MethodToml>,
    #[serde(default)]
    consts: Vec<Spanned<String>>,
    #[serde(default)]
    enums: Vec<EnumToml>,
    #[serde(default)]
    unions: Vec<UnionToml>,
    #[serde(default)]
    resources: Vec<String>,
    settings: Option<SettingsToml>,
    requests: Option<RequestsToml>,
}

fn default_wit() -> String {
    "wit".to_string()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MethodToml {
    signature: Spanned<String>,
    #[serde(default)]
    sink: Vec<String>,
    help: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnumToml {
    name: String,
    cases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnionToml {
    name: String,
    cases: Vec<UnionCaseToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnionCaseToml {
    name: String,
    shape: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsToml {
    name: String,
    #[serde(default)]
    keys: Vec<SettingKeyToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingKeyToml {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    default: toml::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestsToml {
    #[serde(default)]
    read: Vec<String>,
    #[serde(default)]
    write: Vec<String>,
    #[serde(default)]
    connect: Vec<String>,
}

/// The `nvsx.toml` text `text`, where `path` names the file in every refusal.
pub(crate) fn parse(path: &str, text: &str) -> Result<Parsed, String> {
    let at = |span: Option<Range<usize>>, message: &str| match span {
        Some(span) => format!("{path}:{}: {message}", line_of(text, span.start)),
        None => format!("{path}: {message}"),
    };
    let file: Toml = toml::from_str(text).map_err(|err| at(err.span(), err.message()))?;

    let mut methods = Vec::with_capacity(file.methods.len());
    for method in &file.methods {
        let json =
            method_json(method).map_err(|message| at(Some(method.signature.span()), &message))?;
        methods.push(json);
    }
    let mut consts = Vec::with_capacity(file.consts.len());
    for decl in &file.consts {
        consts.push(const_json(decl.get_ref()).map_err(|message| at(Some(decl.span()), &message))?);
    }

    let world = file
        .world
        .unwrap_or_else(|| nvs_ext::load::WORLD.to_string());
    let mut manifest = Map::new();
    manifest.insert("manifest".into(), json!(nvs_ext::manifest::FORMAT));
    manifest.insert("world".into(), json!(world));
    manifest.insert("class".into(), json!(file.class));
    manifest.insert("interface".into(), json!(file.interface));
    manifest.insert("methods".into(), Value::Array(methods));
    manifest.insert("consts".into(), Value::Array(consts));
    let enums = file
        .enums
        .iter()
        .map(|decl| json!({"name": decl.name, "cases": decl.cases}));
    manifest.insert("enums".into(), Value::Array(enums.collect()));
    let unions = file.unions.iter().map(|decl| {
        let cases = decl
            .cases
            .iter()
            .map(|case| json!({"name": case.name, "shape": case.shape}));
        json!({"name": decl.name, "cases": cases.collect::<Vec<_>>()})
    });
    manifest.insert("unions".into(), Value::Array(unions.collect()));
    let resources = file.resources.iter().map(|name| json!({"name": name}));
    manifest.insert("resources".into(), Value::Array(resources.collect()));
    if let Some(settings) = &file.settings {
        let mut keys = Vec::with_capacity(settings.keys.len());
        for key in &settings.keys {
            let default = serde_json::to_value(&key.default).map_err(|err| {
                format!(
                    "{path}: the default of the setting `{}` does not read: {err}",
                    key.name
                )
            })?;
            keys.push(json!({"name": key.name, "type": key.ty, "default": default}));
        }
        manifest.insert(
            "settings".into(),
            json!({"name": settings.name, "keys": keys}),
        );
    }
    if let Some(requests) = &file.requests {
        manifest.insert(
            "requests".into(),
            json!({"read": requests.read, "write": requests.write, "connect": requests.connect}),
        );
    }
    if let Some(memory) = file.memory {
        manifest.insert("memory".into(), json!(memory));
    }

    let bytes = serde_json::to_vec_pretty(&Value::Object(manifest))
        .map_err(|err| format!("{path}: the manifest cannot be written: {err}"))?;
    nvs_ext::manifest::Manifest::parse(&bytes).map_err(|err| format!("{path}: {}", err.0))?;
    Ok(Parsed {
        module: file.module,
        wit: file.wit,
        source: file.source,
        manifest: bytes,
    })
}

/// The one-based line the byte `offset` of `text` is on.
fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

/// One `[[methods]]` entry as the manifest's method object.
fn method_json(method: &MethodToml) -> Result<Value, String> {
    let signature = Signature::parse(method.signature.get_ref())?;
    for name in &method.sink {
        if !signature.params.iter().any(|param| &param.name == name) {
            return Err(format!(
                "`sink` names `{name}`, and `{}` has no parameter `${name}`",
                signature.name
            ));
        }
    }
    let params = signature.params.iter().map(|param| {
        let mut object = Map::new();
        object.insert("name".into(), json!(param.name));
        object.insert("type".into(), json!(param.ty));
        if let Some(default) = &param.default {
            object.insert("default".into(), default.clone());
        }
        if method.sink.contains(&param.name) {
            object.insert("sink".into(), json!(true));
        }
        Value::Object(object)
    });
    let mut object = Map::new();
    object.insert("name".into(), json!(signature.name));
    object.insert("params".into(), Value::Array(params.collect()));
    object.insert("returns".into(), json!(signature.returns));
    object.insert("source".into(), json!(signature.source));
    if let Some(help) = &method.help {
        object.insert("help".into(), json!(help));
    }
    Ok(Value::Object(object))
}

/// A `consts` entry, `[const] type NAME = literal`, as the manifest's constant object.
fn const_json(text: &str) -> Result<Value, String> {
    let text = text.trim();
    let text = text.strip_prefix("const ").unwrap_or(text);
    let (head, value) = split_top(text, '=')
        .filter(|parts| parts.len() == 2)
        .map(|parts| (parts[0].trim(), parts[1].trim()))
        .ok_or_else(|| format!("`{text}` is not a constant like `int LIMIT = 10`"))?;
    let (ty, name) = head
        .rsplit_once(char::is_whitespace)
        .map(|(ty, name)| (ty.trim(), name.trim()))
        .filter(|(ty, name)| !ty.is_empty() && !name.is_empty())
        .ok_or_else(|| format!("the constant `{head}` needs a type before its name"))?;
    let value = literal(value)?;
    Ok(json!({"name": name, "type": ty, "value": value}))
}

/// A method signature, written as Novis text.
struct Signature {
    name: String,
    params: Vec<Param>,
    returns: String,
    source: bool,
}

struct Param {
    name: String,
    ty: String,
    default: Option<Value>,
}

impl Signature {
    fn parse(text: &str) -> Result<Self, String> {
        let shape = || format!("`{text}` is not a signature like `name(int $a): string`");
        let mut rest = text.trim();
        for word in ["public", "static", "function"] {
            if let Some(after) = rest.strip_prefix(word)
                && after.starts_with(char::is_whitespace)
            {
                rest = after.trim_start();
            }
        }
        let open = rest.find('(').ok_or_else(shape)?;
        let name = rest[..open].trim();
        if name.is_empty() || name.contains(char::is_whitespace) {
            return Err(shape());
        }
        let close = matching_paren(rest, open).ok_or_else(shape)?;
        let inner = rest[open + 1..close].trim();
        let params = if inner.is_empty() {
            Vec::new()
        } else {
            split_top(inner, ',')
                .ok_or_else(shape)?
                .into_iter()
                .map(param)
                .collect::<Result<Vec<_>, _>>()?
        };
        let tail = rest[close + 1..].trim();
        let returns = if tail.is_empty() {
            "void"
        } else {
            tail.strip_prefix(':').ok_or_else(shape)?.trim()
        };
        let (returns, source) = match returns.strip_prefix("tainted") {
            Some(after) if after.starts_with(char::is_whitespace) => (after.trim(), true),
            _ => (returns, false),
        };
        if returns.is_empty() {
            return Err(shape());
        }
        Ok(Self {
            name: name.to_string(),
            params,
            returns: returns.to_string(),
            source,
        })
    }
}

/// One parameter, `type $name [= literal]`.
fn param(text: &str) -> Result<Param, String> {
    let text = text.trim();
    let (head, default) = match split_top(text, '=') {
        Some(parts) if parts.len() == 2 => (parts[0].trim(), Some(literal(parts[1].trim())?)),
        Some(parts) if parts.len() == 1 => (text, None),
        _ => return Err(format!("the parameter `{text}` has more than one `=`")),
    };
    let dollar = head
        .rfind('$')
        .ok_or_else(|| format!("the parameter `{text}` is not written like `int $count`"))?;
    let ty = head[..dollar].trim();
    let name = head[dollar + 1..].trim();
    if ty.is_empty() {
        return Err(format!("the parameter `${name}` needs a type"));
    }
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!(
            "the parameter `{text}` is not written like `int $count`"
        ));
    }
    Ok(Param {
        name: name.to_string(),
        ty: ty.to_string(),
        default,
    })
}

/// The index of the `)` that closes the `(` at `open`.
fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (i, c) in text.char_indices().skip_while(|&(i, _)| i < open) {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '<' | '{' | '[' => depth += 1,
            ')' | '>' | '}' | ']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 && c == ')' {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `text` split at each `sep` outside brackets and quotes, or `None` when a bracket or a quote is
/// left open.
fn split_top(text: &str, sep: char) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '<' | '{' | '[' => depth += 1,
            ')' | '>' | '}' | ']' => depth = depth.checked_sub(1)?,
            _ if c == sep && depth == 0 => {
                parts.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    (depth == 0 && quote.is_none()).then(|| {
        parts.push(&text[start..]);
        parts
    })
}

/// A Novis literal as its JSON value.
fn literal(text: &str) -> Result<Value, String> {
    let text = text.trim();
    let not = || {
        format!(
            "`{text}` is not a literal `nvsx.toml` can write: `null`, `true`, `false`, a number, a quoted string or a list in `[...]`"
        )
    };
    match text {
        "null" => return Ok(Value::Null),
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }
    if let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if inner.trim().is_empty() {
            return Ok(Value::Array(Vec::new()));
        }
        let items = split_top(inner, ',').ok_or_else(not)?;
        return items
            .into_iter()
            .map(literal)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array);
    }
    if let Some(quote) = text.chars().next().filter(|c| *c == '"' || *c == '\'') {
        let inner = text[1..]
            .strip_suffix(quote)
            .filter(|_| text.len() >= 2)
            .ok_or_else(not)?;
        return unescape(inner, quote).map(Value::String).ok_or_else(not);
    }
    if let Ok(int) = text.replace('_', "").parse::<i64>() {
        return Ok(json!(int));
    }
    text.replace('_', "")
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite() && text.contains(|c: char| c.is_ascii_digit()))
        .and_then(Number::from_f64)
        .map(Value::Number)
        .ok_or_else(not)
}

/// The text of a string literal between its quotes, or `None` when an escape does not read or a
/// quote is not escaped.
fn unescape(inner: &str, quote: char) -> Option<String> {
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == quote {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        let next = chars.next()?;
        match (quote, next) {
            (_, '\\') => out.push('\\'),
            (_, q) if q == quote => out.push(q),
            ('"', 'n') => out.push('\n'),
            ('"', 't') => out.push('\t'),
            ('"', 'r') => out.push('\r'),
            ('"', '$') => out.push('$'),
            ('\'', other) => {
                out.push('\\');
                out.push(other);
            }
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEO: &str = r#"class = "Shop\\Geo"
interface = "shop:geo/api"
module = "geo.wasm"
source = "nvs"
memory = 67108864
resources = ["Route"]
consts = ["const float EARTH_RADIUS_KM = 6371.0"]

[[methods]]
signature = "public static function distanceKm(string $from, string $to, bool $round = false): float"
sink = ["from"]
help = "The distance between two places, in kilometres."

[[methods]]
signature = "lookup(string $name, ?string $country = null): tainted ?string"

[[enums]]
name = "Unit"
cases = ["Metres", "NauticalMiles"]

[settings]
name = "geo"
keys = [{ name = "precision", type = "int", default = 6 }]

[requests]
read = ["data/geo/"]
"#;

    fn manifest(text: &str) -> Value {
        let parsed = parse("nvsx.toml", text).expect("the file reads");
        serde_json::from_slice(&parsed.manifest).expect("the manifest is JSON")
    }

    #[test]
    fn every_key_is_written_into_the_manifest_the_loader_reads() {
        let parsed = parse("nvsx.toml", GEO).expect("the file reads");
        assert_eq!(parsed.module, "geo.wasm");
        assert_eq!(parsed.wit, "wit");
        assert_eq!(parsed.source.as_deref(), Some("nvs"));
        let read = nvs_ext::manifest::Manifest::parse(&parsed.manifest).expect("it loads");
        assert_eq!(read.class, "Shop\\Geo");
        assert_eq!(read.world, nvs_ext::load::WORLD);
        assert_eq!(read.memory, Some(67_108_864));
        assert_eq!(read.consts[0].name, "EARTH_RADIUS_KM");
        assert_eq!(read.consts[0].ty, "float");
        assert_eq!(read.resources[0].name, "Route");
        assert_eq!(read.enums[0].cases, ["Metres", "NauticalMiles"]);
        assert_eq!(read.requests.read, ["data/geo/"]);
        assert_eq!(read.settings.as_ref().map(|s| s.name.as_str()), Some("geo"));

        let distance = &read.methods[0];
        assert_eq!(distance.name, "distanceKm");
        assert_eq!(distance.returns, "float");
        assert!(!distance.source);
        assert_eq!(
            distance.help.as_deref(),
            Some("The distance between two places, in kilometres.")
        );
        let names: Vec<_> = distance
            .params
            .iter()
            .map(|p| (p.name.as_str(), p.ty.as_str(), p.sink))
            .collect();
        assert_eq!(
            names,
            [
                ("from", "string", true),
                ("to", "string", false),
                ("round", "bool", false)
            ]
        );
        assert_eq!(distance.params[2].default, Some(Value::Bool(false)));
        assert_eq!(distance.params[0].default, None);

        let lookup = &read.methods[1];
        assert_eq!(lookup.returns, "?string");
        assert!(lookup.source);
        assert_eq!(lookup.params[1].default, Some(Value::Null));
    }

    #[test]
    fn a_missing_return_type_is_void_and_literals_read_as_their_json() {
        let json = manifest(
            "class = \"Shop\\\\Geo\"\ninterface = \"shop:geo/api\"\nmodule = \"m.wasm\"\n\
             [[methods]]\nsignature = \"tag(string $a = \\\"x,\\\\\\\"y\\\", int $b = -3, float $c = 1.5, array<string> $d = ['p', \\\"q\\\"])\"\n",
        );
        let method = &json["methods"][0];
        assert_eq!(method["returns"], "void");
        let defaults: Vec<_> = method["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["default"].clone())
            .collect();
        assert_eq!(
            defaults,
            [json!("x,\"y"), json!(-3), json!(1.5), json!(["p", "q"])]
        );
        assert_eq!(method["params"][3]["type"], "array<string>");
    }

    #[test]
    fn an_unknown_key_is_refused_with_the_file_and_its_line() {
        let err = parse("geo/nvsx.toml", "class = \"Shop\\\\Geo\"\ninterface = \"shop:geo/api\"\nmodule = \"m.wasm\"\ncolour = 1\n")
            .expect_err("an unknown key");
        assert!(err.starts_with("geo/nvsx.toml:4: "), "{err}");
        assert!(err.contains("colour"), "{err}");

        let err = parse("nvsx.toml", "class = \"A\"\ninterface = \"a:b/c\"\nmodule = \"m.wasm\"\n[[methods]]\nsignature = \"f()\"\nsinks = []\n")
            .expect_err("an unknown method key");
        assert!(err.starts_with("nvsx.toml:6: "), "{err}");
    }

    #[test]
    fn a_signature_that_does_not_read_names_its_line() {
        let head = "class = \"A\"\ninterface = \"a:b/c\"\nmodule = \"m.wasm\"\n[[methods]]\n";
        for (signature, says) in [
            ("signature = \"f(int a)\"", "`int $count`"),
            ("signature = \"f(int $a\"", "is not a signature"),
            ("signature = \"f() float\"", "is not a signature"),
            ("signature = \"f(int $a = nope)\"", "is not a literal"),
            (
                "signature = \"f(int $a)\"\nsink = [\"b\"]",
                "no parameter `$b`",
            ),
        ] {
            let err = parse("nvsx.toml", &format!("{head}{signature}\n")).expect_err(signature);
            assert!(err.starts_with("nvsx.toml:5: "), "{err}");
            assert!(err.contains(says), "{signature}: {err}");
        }
    }

    #[test]
    fn a_manifest_the_loader_refuses_is_refused_naming_the_file() {
        let err = parse("nvsx.toml", "class = \"Shop\\\\Geo\"\ninterface = \"shop:geo/api\"\nmodule = \"m.wasm\"\nworld = \"one\"\n")
            .expect_err("a version that does not read");
        assert!(err.starts_with("nvsx.toml: "), "{err}");
        let err = parse("nvsx.toml", "class = \"Shop\\\\Geo\"\ninterface = \"shop:geo/api\"\nmodule = \"m.wasm\"\n[[methods]]\nsignature = \"f(): secret string\"\n")
            .expect_err("a secret return");
        assert!(err.contains("secret"), "{err}");
    }
}
