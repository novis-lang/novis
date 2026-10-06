//! Loading a `.nvsx` from its `[[extension]]` entry: `rule:packaging/extension-loading-is-root-controlled`'s
//! checks, in ADR 0246 § 4's order, each refusal naming the entry.
//!
//! [`Loader::load`] reads the file and refuses, in this order:
//!
//! 1. a file whose SHA-256 differs from the entry's pin, compared case-insensitively because the
//!    configuration accepts a pin in either case;
//! 2. bytes wasmtime does not compile as a component;
//! 3. a component with no `nvs.manifest` section, a manifest or a source section that does not
//!    read, and a component carrying no `nvs.source` is one with no source files;
//! 4. a manifest built against another major of the world or a newer minor, naming both versions —
//!    checked before the imports, so a newer component is refused by its version and not by the
//!    first import this host lacks;
//! 5. an import outside the world `wit/nvs-ext/world.wit` names ([`WORLD_IMPORTS`]), naming it;
//! 6. a class under `Core\` or `Novis\`, whatever the case of the first segment;
//! 7. a method whose export is missing from the manifest's interface, or whose parameters or result
//!    are not the WIT types `rule:packaging/a-value-crosses-as-its-wit-type`'s table gives its
//!    Novis types, and a Novis type the table does not hold.
//!
//! [`Set::insert`] adds the last check, a class another extension of the set already declares, so
//! the set's classes are unique and its hash does not depend on the entries' order.
//!
//! **Which Novis types the check reads.** The structural ones: `bool`, `int`, `uint`, `float`,
//! `string`, `bytes`, `array<T>`, `array<K, V>`, `?T`, a shape `{a: T, b?: U}` (its keys in
//! kebab-case, an optional key an `option`), `mixed` as a `borrow` of a resource, and `void` as a
//! return. A named type — an enum, a `Core` value class, a union of shapes, a `resource` — is
//! refused as outside the table until the manifest carries what it names, which is the compiler
//! goal's to add. The `err` side of every result is the world's `error` variant, read by its three
//! cases.
//!
//! **A WASI import matches by interface and `0.2`**, any patch: wasmtime's linker resolves a `0.2.x`
//! import to the `0.2` release the host defines. An `nvs:ext` import matches by interface, under
//! the version rule the manifest's own `world` is held to.
//!
//! **The compiled form is cached behind a seam.** A loader given a [`ModuleCache`] looks the
//! component up under a [`CacheKey`] — the file's pin and [`Loader::environment`], a SHA-256 of
//! wasmtime's own compatibility hash (its version, the engine's configuration and the target) — and
//! deserializes it on a hit rather than compiling. A miss compiles and stores the serialized form.
//! The bytes stored are a SHA-256 of wasmtime's serialized component followed by that component, so
//! an entry whose checksum does not match, or that wasmtime does not accept, is a miss and is
//! overwritten by the next store: `rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`. The
//! store itself is `nvs-cli`'s artifact cache, which owns the directory's ownership check
//! (`rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache`); this crate never touches a file
//! for it. Deserializing is the crate's one `unsafe` call, and [`Loader::cached`] states why it holds.
//!
//! Cost: one SHA-256 of the file, one wasmtime compile or one deserialize and SHA-256 of the
//! compiled form, and one walk of the component's type, at boot and at reload. Nothing here runs on
//! a request path.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};
use wasmtime::Engine;
use wasmtime::component::types::{ComponentFunc, ComponentItem};
use wasmtime::component::{Component, types::Type};

use crate::manifest::{Manifest, Method, WorldVersion};
use crate::section;
use crate::source::Source;

/// The `nvs:ext` world this host implements.
pub const WORLD: WorldVersion = WorldVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

/// The interfaces the world lets a component import, without their versions. The list is
/// `wit/nvs-ext/world.wit`'s, and `tests/load.rs` holds the two equal.
pub const WORLD_IMPORTS: &[&str] = &[
    "nvs:ext/types",
    "nvs:ext/log",
    "nvs:ext/settings",
    "wasi:cli/environment",
    "wasi:cli/exit",
    "wasi:cli/stdin",
    "wasi:cli/stdout",
    "wasi:cli/stderr",
    "wasi:clocks/monotonic-clock",
    "wasi:clocks/wall-clock",
    "wasi:random/random",
    "wasi:random/insecure",
    "wasi:random/insecure-seed",
    "wasi:filesystem/types",
    "wasi:filesystem/preopens",
    "wasi:http/types",
    "wasi:http/outgoing-handler",
    "wasi:io/error",
    "wasi:io/poll",
    "wasi:io/streams",
];

/// One `[[extension]]` entry: the file to load and the pin it must match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The `.nvsx` file, as the entry's `path` resolves.
    pub path: PathBuf,
    /// The entry's `sha256`, 64 hexadecimal digits in either case.
    pub sha256: String,
    /// The entry's optional `memory` ceiling, in bytes.
    pub memory: Option<u64>,
}

/// Why an entry does not load. `entry` is the entry's path, and the caller that knows the
/// configuration file adds its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The entry's path, as written.
    pub entry: String,
    /// What is wrong with it.
    pub reason: String,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the extension `{}` does not load: {}",
            self.entry, self.reason
        )
    }
}

impl std::error::Error for Refused {}

/// One loaded extension: its compiled component, its manifest and its source files.
#[derive(Clone)]
pub struct Extension {
    /// The file it was loaded from.
    pub path: PathBuf,
    /// The file's SHA-256, lower-case hexadecimal.
    pub sha256: String,
    /// The entry's `memory` ceiling, in bytes. A guest's linear memory stays under it and under
    /// the manifest's `memory`.
    pub memory: Option<u64>,
    /// The compiled component, shared by every core.
    pub component: Component,
    /// The class it declares and that class's signatures.
    pub manifest: Manifest,
    /// The Novis source it carries.
    pub source: Source,
}

impl fmt::Debug for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extension")
            .field("path", &self.path)
            .field("sha256", &self.sha256)
            .field("class", &self.manifest.class)
            .finish_non_exhaustive()
    }
}

/// A set of loaded extensions, no two declaring one class.
#[derive(Debug, Clone, Default)]
pub struct Set {
    extensions: Vec<Extension>,
}

impl Set {
    /// Adds `extension`, or refuses it when another extension of the set declares its class.
    /// Class names compare without regard to case, as Novis names do.
    pub fn insert(&mut self, extension: Extension) -> Result<(), Refused> {
        if let Some(other) = self.extensions.iter().find(|other| {
            other
                .manifest
                .class
                .eq_ignore_ascii_case(&extension.manifest.class)
        }) {
            return Err(refused(
                &extension.path,
                format!(
                    "the class `{}` is already declared by the extension `{}`",
                    extension.manifest.class,
                    other.path.display()
                ),
            ));
        }
        self.extensions.push(extension);
        Ok(())
    }

    /// The extensions, in the order they were added.
    pub fn extensions(&self) -> &[Extension] {
        &self.extensions
    }
}

/// The SHA-256 of `bytes` as an entry's pin: 64 lower-case hexadecimal digits.
pub fn pin(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Where a compiled component is kept between loads: `nvs-cli`'s artifact cache, or a test's map.
///
/// Neither call reports a failure. A `get` that cannot read is a miss, and a `put` that cannot
/// write is dropped, because the loader's next move is the compile it would have done anyway.
pub trait ModuleCache: Send + Sync {
    /// The bytes last stored under `key`, if any.
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>>;
    /// Stores `bytes` under `key`, replacing what was there.
    fn put(&self, key: &CacheKey, bytes: &[u8]);
}

/// What a compiled component is stored under: the file it was compiled from and the engine that
/// compiled it. Either one differing is a different key, so a component compiled by another engine
/// is never looked at.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The file's SHA-256, lower-case hexadecimal.
    pub pin: String,
    /// [`Loader::environment`] of the loader that compiled it.
    pub environment: String,
}

/// Feeds wasmtime's compatibility hash into a SHA-256, so the environment is the same digest in
/// every process of one build.
struct Sha256Hasher(Sha256);

impl Hasher for Sha256Hasher {
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(&self) -> u64 {
        let digest = self.0.clone().finalize();
        u64::from_le_bytes(digest[..8].try_into().expect("a SHA-256 has eight bytes"))
    }
}

/// Loads extensions into one engine, against one version of the world.
#[derive(Clone)]
pub struct Loader {
    engine: Engine,
    world: WorldVersion,
    environment: String,
    cache: Option<Arc<dyn ModuleCache>>,
}

impl fmt::Debug for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Loader")
            .field("world", &self.world)
            .field("environment", &self.environment)
            .field("cached", &self.cache.is_some())
            .finish_non_exhaustive()
    }
}

impl Loader {
    /// A loader compiling into `engine`, implementing [`WORLD`], with no cache.
    pub fn new(engine: &Engine) -> Self {
        let mut hasher = Sha256Hasher(Sha256::new());
        engine.precompile_compatibility_hash().hash(&mut hasher);
        Self {
            engine: engine.clone(),
            world: WORLD,
            environment: hex(&hasher.0.finalize()),
            cache: None,
        }
    }

    /// The same loader, implementing the world version `world` instead.
    #[must_use]
    pub fn with_world(mut self, world: WorldVersion) -> Self {
        self.world = world;
        self
    }

    /// The same loader, keeping compiled components in `cache`.
    #[must_use]
    pub fn with_cache(mut self, cache: Arc<dyn ModuleCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// The engine's environment: a SHA-256, in lower-case hexadecimal, of everything wasmtime
    /// requires to match before it loads a component another engine compiled.
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// The component `bytes` compile to, from the cache when it holds a good entry under `sha256`,
    /// and stored there when it did not.
    fn compile(&self, sha256: &str, bytes: &[u8]) -> wasmtime::Result<Component> {
        let Some(cache) = &self.cache else {
            return Component::new(&self.engine, bytes);
        };
        let key = CacheKey {
            pin: sha256.to_owned(),
            environment: self.environment.clone(),
        };
        if let Some(component) = cache.get(&key).and_then(|stored| self.cached(&stored)) {
            return Ok(component);
        }
        let component = Component::new(&self.engine, bytes)?;
        if let Ok(serialized) = component.serialize() {
            let mut stored = Sha256::digest(&serialized).to_vec();
            stored.extend_from_slice(&serialized);
            cache.put(&key, &stored);
        }
        Ok(component)
    }

    /// The component a stored entry carries, or [`None`] when its checksum does not match or
    /// wasmtime does not accept it.
    #[expect(
        unsafe_code,
        reason = "`Component::deserialize` is unsafe because it loads machine code without \
                  validating it; the block below states why these bytes are ones this engine wrote"
    )]
    fn cached(&self, stored: &[u8]) -> Option<Component> {
        let (checksum, serialized) = stored.split_at_checked(32)?;
        if Sha256::digest(serialized).as_slice() != checksum {
            return None;
        }
        // SAFETY: the entry is under a key carrying this engine's environment, and its checksum
        // matches the bytes `Component::serialize` returned when it was stored, so it is unchanged
        // output of an engine wasmtime calls compatible, which it checks again from the bytes' own
        // header. Who can write the store is bounded by its owner's check on the directory, the
        // same one the artifact cache's native code rests on.
        unsafe { Component::deserialize(&self.engine, serialized) }.ok()
    }

    /// Every entry of `entries`, loaded into one set, or the first entry that does not load.
    pub fn load_set(&self, entries: &[Entry]) -> Result<Set, Refused> {
        let mut set = Set::default();
        for entry in entries {
            set.insert(self.load(entry)?)?;
        }
        Ok(set)
    }

    /// The extension `entry` names, read from its file.
    pub fn load(&self, entry: &Entry) -> Result<Extension, Refused> {
        let bytes = std::fs::read(&entry.path)
            .map_err(|err| refused(&entry.path, format!("the file does not read: {err}")))?;
        self.load_bytes(entry, &bytes)
    }

    /// The extension `entry` names, from the file's `bytes`.
    pub fn load_bytes(&self, entry: &Entry, bytes: &[u8]) -> Result<Extension, Refused> {
        let refuse = |reason: String| refused(&entry.path, reason);
        let sha256 = pin(bytes);
        if !sha256.eq_ignore_ascii_case(&entry.sha256) {
            return Err(refuse(format!(
                "the file's sha256 is {sha256}, and the entry pins {}",
                entry.sha256
            )));
        }
        let component = self
            .compile(&sha256, bytes)
            .map_err(|err| refuse(format!("the file is not a valid component: {err:#}")))?;
        let sections = section::read(bytes).map_err(|err| refuse(err.0))?;
        let manifest = sections
            .manifest
            .ok_or_else(|| refuse("the component has no `nvs.manifest` section".to_owned()))?;
        let manifest = Manifest::parse(manifest).map_err(|err| refuse(err.0))?;
        let source = match sections.source {
            Some(source) => Source::parse(source).map_err(|err| refuse(err.0))?,
            None => Source {
                source: crate::source::FORMAT,
                files: Vec::new(),
            },
        };
        if !self.admits(manifest.world) {
            return Err(refuse(format!(
                "it was built against the world nvs:ext@{}, and this host implements nvs:ext@{}",
                manifest.world, self.world
            )));
        }
        let ty = component.component_type();
        for (name, _) in ty.imports(&self.engine) {
            if !self.imports(name) {
                return Err(refuse(format!(
                    "it imports `{name}`, which the world nvs:ext@{} does not offer",
                    self.world
                )));
            }
        }
        let namespace = manifest.class.split('\\').next().unwrap_or_default();
        if ["Core", "Novis"]
            .iter()
            .any(|reserved| namespace.eq_ignore_ascii_case(reserved))
        {
            return Err(refuse(format!(
                "the class `{}` is under `{namespace}\\`, which only Novis declares in",
                manifest.class
            )));
        }
        let interface = ty
            .exports(&self.engine)
            .find(|(name, _)| export_names(name, &manifest.interface))
            .map(|(_, export)| export.ty);
        let Some(ComponentItem::ComponentInstance(interface)) = interface else {
            return Err(refuse(format!(
                "the component exports no instance `{}`",
                manifest.interface
            )));
        };
        for method in &manifest.methods {
            let export = method.export_name();
            let func = interface
                .get_export(&self.engine, &export)
                .map(|export| export.ty);
            let Some(ComponentItem::ComponentFunc(func)) = func else {
                return Err(refuse(format!(
                    "the method `{}` has no function `{export}` in `{}`",
                    method.name, manifest.interface
                )));
            };
            check_signature(method, &func).map_err(|reason| {
                refuse(format!(
                    "the method `{}` does not match `{}#{export}`: {reason}",
                    method.name, manifest.interface
                ))
            })?;
        }
        Ok(Extension {
            path: entry.path.clone(),
            sha256,
            memory: entry.memory,
            component,
            manifest,
            source,
        })
    }

    /// Whether a component built against `built` loads on this host: the same major, and a minor
    /// no newer.
    fn admits(&self, built: WorldVersion) -> bool {
        built.major == self.world.major && built.minor <= self.world.minor
    }

    /// Whether the import `name` is one the world offers.
    fn imports(&self, name: &str) -> bool {
        let (interface, version) = name.split_once('@').unwrap_or((name, ""));
        if !WORLD_IMPORTS.contains(&interface) {
            return false;
        }
        if interface.starts_with("wasi:") {
            return version.strip_prefix("0.2.").is_some_and(|patch| {
                !patch.is_empty() && patch.bytes().all(|b| b.is_ascii_digit())
            });
        }
        WorldVersion::try_from(version.to_owned()).is_ok_and(|version| self.admits(version))
    }
}

fn refused(path: &Path, reason: String) -> Refused {
    Refused {
        entry: path.display().to_string(),
        reason,
    }
}

/// Whether the export `name` is the interface `interface`, at any version.
pub(crate) fn export_names(name: &str, interface: &str) -> bool {
    name.split_once('@').map_or(name, |(bare, _)| bare) == interface
}

/// Whether `func` is the WIT function `method`'s Novis signature gives.
fn check_signature(method: &Method, func: &ComponentFunc) -> Result<(), String> {
    let params: Vec<(&str, Type)> = func.params().collect();
    if params.len() != method.params.len() {
        return Err(format!(
            "the manifest has {} parameter(s), and the export has {}",
            method.params.len(),
            params.len()
        ));
    }
    for (param, (name, wit)) in method.params.iter().zip(&params) {
        let expected = crate::kebab(&param.name);
        if *name != expected {
            return Err(format!(
                "the parameter `{}` is `{name}` in the export, and `{expected}` was expected",
                param.name
            ));
        }
        let ty = NovisType::parse(&param.ty)?;
        if !ty.matches(wit) {
            return Err(format!(
                "the parameter `{}` is `{}`, which does not cross as the export's type",
                param.name, param.ty
            ));
        }
    }
    let results: Vec<Type> = func.results().collect();
    let [Type::Result(result)] = results.as_slice() else {
        return Err("the export does not return `result<T, error>`".to_owned());
    };
    let returns = (method.returns != "void")
        .then(|| NovisType::parse(&method.returns))
        .transpose()?;
    let ok = match (&returns, result.ok()) {
        (None, None) => true,
        (Some(ty), Some(wit)) => ty.matches(&wit),
        _ => false,
    };
    if !ok {
        return Err(format!(
            "it returns `{}`, which does not cross as the export's result",
            method.returns
        ));
    }
    if !result.err().is_some_and(|err| is_error(&err)) {
        return Err("the export's error is not the world's `error` variant".to_owned());
    }
    Ok(())
}

/// Whether `ty` is `nvs:ext/types`'s `error`: a variant of `invalid`, `parse` and `runtime`, each
/// carrying a string.
fn is_error(ty: &Type) -> bool {
    let Type::Variant(variant) = ty else {
        return false;
    };
    let cases: Vec<_> = variant.cases().collect();
    cases.len() == 3
        && ["invalid", "parse", "runtime"]
            .iter()
            .zip(&cases)
            .all(|(name, case)| case.name == *name && matches!(case.ty, Some(Type::String)))
}

/// A Novis type of an extension signature, parsed from the manifest's text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NovisType {
    Bool,
    Int,
    Uint,
    Float,
    String,
    Bytes,
    Mixed,
    List(Box<NovisType>),
    Keyed(Box<NovisType>, Box<NovisType>),
    Optional(Box<NovisType>),
    Shape(Vec<(String, bool, NovisType)>),
}

impl NovisType {
    /// The type `text` writes, or why it is outside the table.
    fn parse(text: &str) -> Result<Self, String> {
        let mut parser = TypeParser { text, at: 0 };
        let ty = parser.ty();
        parser.skip_space();
        match ty {
            Some(ty) if parser.at == text.len() => Ok(ty),
            _ => Err(format!(
                "the type `{text}` is not one an extension signature can carry"
            )),
        }
    }

    /// Whether a value of this type crosses as `wit`.
    fn matches(&self, wit: &Type) -> bool {
        match (self, wit) {
            (Self::Bool, Type::Bool)
            | (Self::Int, Type::S64)
            | (Self::Uint, Type::U64)
            | (Self::Float, Type::Float64)
            | (Self::String, Type::String)
            | (Self::Mixed, Type::Borrow(_)) => true,
            (Self::Bytes, Type::List(list)) => matches!(list.ty(), Type::U8),
            (Self::List(item), Type::List(list)) => item.matches(&list.ty()),
            (Self::Keyed(key, value), Type::List(list)) => match list.ty() {
                Type::Tuple(tuple) => {
                    let types: Vec<Type> = tuple.types().collect();
                    matches!(types.as_slice(), [k, v] if key.matches(k) && value.matches(v))
                }
                _ => false,
            },
            (Self::Optional(inner), Type::Option(option)) => inner.matches(&option.ty()),
            (Self::Shape(fields), Type::Record(record)) => {
                let wit: Vec<_> = record.fields().collect();
                wit.len() == fields.len()
                    && fields.iter().zip(&wit).all(|((name, optional, ty), field)| {
                        field.name == crate::kebab(name)
                            && if *optional {
                                matches!(&field.ty, Type::Option(option) if ty.matches(&option.ty()))
                            } else {
                                ty.matches(&field.ty)
                            }
                    })
            }
            _ => false,
        }
    }
}

/// A recursive-descent reader over a type's text.
struct TypeParser<'a> {
    text: &'a str,
    at: usize,
}

impl TypeParser<'_> {
    fn skip_space(&mut self) {
        let rest = &self.text[self.at..];
        self.at += rest.len() - rest.trim_start().len();
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        if self.text[self.at..].starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    fn word(&mut self) -> Option<&str> {
        self.skip_space();
        let rest = &self.text[self.at..];
        let len = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if len == 0 {
            return None;
        }
        self.at += len;
        Some(&rest[..len])
    }

    fn ty(&mut self) -> Option<NovisType> {
        if self.eat("?") {
            return Some(NovisType::Optional(Box::new(self.ty()?)));
        }
        if self.eat("{") {
            let mut fields = Vec::new();
            if !self.eat("}") {
                loop {
                    let name = self.word()?.to_owned();
                    let optional = self.eat("?");
                    if !self.eat(":") {
                        return None;
                    }
                    fields.push((name, optional, self.ty()?));
                    if self.eat("}") {
                        break;
                    }
                    if !self.eat(",") {
                        return None;
                    }
                }
            }
            return Some(NovisType::Shape(fields));
        }
        Some(match self.word()? {
            "bool" => NovisType::Bool,
            "int" => NovisType::Int,
            "uint" => NovisType::Uint,
            "float" => NovisType::Float,
            "string" => NovisType::String,
            "bytes" => NovisType::Bytes,
            "mixed" => NovisType::Mixed,
            "array" => {
                if !self.eat("<") {
                    return None;
                }
                let first = self.ty()?;
                let ty = if self.eat(",") {
                    NovisType::Keyed(Box::new(first), Box::new(self.ty()?))
                } else {
                    NovisType::List(Box::new(first))
                };
                if !self.eat(">") {
                    return None;
                }
                ty
            }
            _ => return None,
        })
    }
}
