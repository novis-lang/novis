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
//! 5. an import outside the world `wit/nvs-ext/world.wit` names ([`WORLD_IMPORTS`]), naming it,
//!    unless it is one of the refusing stubs [`crate::wasi::STUBBED`] names;
//! 6. a class under `Core\` or `Novis\`, whatever the case of the first segment, and then a source
//!    file whose `namespace` is not under the class's own, naming the file ([`Source::outside`]);
//! 7. a method whose export is missing from the manifest's interface, or whose parameters or result
//!    are not the WIT types `rule:packaging/a-value-crosses-as-its-wit-type`'s table gives its
//!    Novis types, and a Novis type the table does not hold.
//!
//! [`Set::insert`] adds the last check, a class another extension of the set already declares, so
//! the set's classes are unique and its hash does not depend on the entries' order.
//!
//! **A built-in component skips check 1 and the reserved half of check 6.** [`Loader::builtins`]
//! reads each one's sections and checks its world, and compiles nothing. Its first
//! [`Builtin::extension`] compiles it, under the digest `build.rs` took, and runs checks 5 to 7 with
//! `Novis\` admitted, because the binary is its pin and `Novis\` is reserved for it. An
//! `[[extension]]` entry declaring a `Novis\` class is still refused, so no entry can shadow one.
//!
//! **Which Novis types the check reads.** The structural ones: `bool`, `int`, `uint`, `float`,
//! `string`, `bytes`, `array<T>`, `array<K, V>`, `?T`, a shape `{a: T, b?: U}` (its keys in
//! kebab-case, an optional key an `option`), `mixed` as a `borrow` of a resource, and `void` as a
//! return. Then the named ones the manifest resolves (`Manifest::novis_type`): an enum as an
//! `enum` of its cases in kebab-case and in order, a closed union of shapes as a `variant` whose
//! cases are its cases in kebab-case, each carrying its shape's record, and a `Core` value class as
//! a record with exactly its fields from `crate::types::CORE_CLASSES`. The record is matched by
//! its structure, because the conversion reads nothing else, so a guest that declares an equal
//! record of its own loads too. A resource is the one the interface exports under its name in
//! kebab-case, matched by its type and not its structure: an `own` of it in a result, which the
//! request keeps, and a `borrow` of it in a parameter, so passing it never gives it away. The `err`
//! side of every result is the world's `error` variant, read by its three cases.
//!
//! **A WASI import matches by interface and `0.2`**, any patch: wasmtime's linker resolves a `0.2.x`
//! import to the `0.2` release the host defines. Of the world's WASI interfaces, only those
//! [`crate::wasi::LINKED`] names load, because an interface the host does not define would fail
//! at instantiation instead. An `nvs:ext` import matches by interface, under the version rule the
//! manifest's own `world` is held to.
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
use std::sync::{Arc, OnceLock};

use nvs_config::extension::Granted;
use sha2::{Digest, Sha256};
use wasmtime::Engine;
use wasmtime::component::ResourceType;
use wasmtime::component::types::{ComponentFunc, ComponentItem, Record};
use wasmtime::component::{Component, types::Type};

use crate::manifest::{Manifest, Method, WorldVersion};
use crate::section;
use crate::source::Source;
use crate::types::{Field, NovisType};
use crate::wasi;

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
    /// The entry's `grants`, every root absolute. Empty grants nothing.
    pub grants: Granted,
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
    /// The entry's `grants`: the operator's part of what the guest may reach
    /// (`rule:security/extension-grants-are-an-intersection`).
    pub grants: Granted,
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
        let sha256 = pin(bytes);
        if !sha256.eq_ignore_ascii_case(&entry.sha256) {
            return Err(refused(
                &entry.path,
                format!(
                    "the file's sha256 is {sha256}, and the entry pins {}",
                    entry.sha256
                ),
            ));
        }
        let component = self.compile(&sha256, bytes).map_err(|err| {
            refused(
                &entry.path,
                format!("the file is not a valid component: {err:#}"),
            )
        })?;
        let (manifest, source) = sections(&entry.path, bytes)?;
        self.check(&entry.path, &component, &manifest, &source, Reserve::Refuse)?;
        Ok(Extension {
            path: entry.path.clone(),
            sha256,
            memory: entry.memory,
            grants: entry.grants.clone(),
            component,
            manifest,
            source,
        })
    }

    /// Every built-in component ([`crate::builtin`]), its sections read and its version checked,
    /// and none of them compiled. [`Builtin::extension`] compiles one on its first call.
    ///
    /// # Errors
    ///
    /// The first built-in component whose sections do not read, or whose world this loader does
    /// not implement. Either one is a broken build.
    pub fn builtins(&self) -> Result<Vec<Builtin>, Refused> {
        BUILTINS
            .iter()
            .map(|&(name, bytes, sha256)| self.builtin(name, bytes, sha256))
            .collect()
    }

    /// The built-in component `name`, from the `bytes` the binary carries and the digest the build
    /// took of them.
    fn builtin(
        &self,
        name: &str,
        bytes: &'static [u8],
        sha256: &[u8; 32],
    ) -> Result<Builtin, Refused> {
        let path = PathBuf::from(format!("built-in {name}.nvsx"));
        let (manifest, source) = sections(&path, bytes)?;
        if !self.admits(manifest.world) {
            return Err(refused(&path, self.mismatch(manifest.world)));
        }
        Ok(Builtin {
            path,
            bytes,
            sha256: hex(sha256),
            manifest,
            source,
            loader: self.clone(),
            extension: OnceLock::new(),
        })
    }

    /// Why a component built against `built` does not load here.
    fn mismatch(&self, built: WorldVersion) -> String {
        format!(
            "it was built against the world nvs:ext@{built}, and this host implements nvs:ext@{}",
            self.world
        )
    }

    /// Checks 4 to 7 of the module doc, over a compiled `component` and its sections. `reserve`
    /// says whether the class may be under `Core\` or `Novis\`.
    fn check(
        &self,
        path: &Path,
        component: &Component,
        manifest: &Manifest,
        source: &Source,
        reserve: Reserve,
    ) -> Result<(), Refused> {
        let refuse = |reason: String| refused(path, reason);
        if !self.admits(manifest.world) {
            return Err(refuse(self.mismatch(manifest.world)));
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
        if let Reserve::Refuse = reserve
            && let Some(reason) = reserved(manifest)
        {
            return Err(refuse(reason));
        }
        if let Some(reason) = outside(source, manifest) {
            return Err(refuse(reason));
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
            let resource = |name: &str| match interface
                .get_export(&self.engine, &crate::kebab(name))
                .map(|export| export.ty)
            {
                Some(ComponentItem::Resource(ty)) => Some(ty),
                _ => None,
            };
            check_signature(manifest, method, &func, &resource).map_err(|reason| {
                refuse(format!(
                    "the method `{}` does not match `{}#{export}`: {reason}",
                    method.name, manifest.interface
                ))
            })?;
        }
        Ok(())
    }

    /// Whether a component built against `built` loads on this host: the same major, and a minor
    /// no newer.
    fn admits(&self, built: WorldVersion) -> bool {
        built.major == self.world.major && built.minor <= self.world.minor
    }

    /// Whether the import `name` is one the world offers.
    fn imports(&self, name: &str) -> bool {
        let (interface, version) = name.split_once('@').unwrap_or((name, ""));
        let stubbed = wasi::STUBBED.contains(&interface);
        if !stubbed && !WORLD_IMPORTS.contains(&interface) {
            return false;
        }
        if interface.starts_with("wasi:") {
            if !stubbed && !wasi::LINKED.contains(&interface) {
                return false;
            }
            return version.strip_prefix("0.2.").is_some_and(|patch| {
                !patch.is_empty() && patch.bytes().all(|b| b.is_ascii_digit())
            });
        }
        WorldVersion::try_from(version.to_owned()).is_ok_and(|version| self.admits(version))
    }
}

/// Whether a class under `Core\` or `Novis\` is refused: always for an `[[extension]]` entry, and
/// never for a built-in component, whose classes are the ones `Novis\` is reserved for.
#[derive(Clone, Copy)]
enum Reserve {
    Refuse,
    Admit,
}

/// The manifest and the source a component's `bytes` carry, or why they do not read.
fn sections(path: &Path, bytes: &[u8]) -> Result<(Manifest, Source), Refused> {
    let refuse = |reason: String| refused(path, reason);
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
    Ok((manifest, source))
}

/// One built-in component: its bytes, which the binary carries, its manifest and its source, and
/// its compiled form once something calls it
/// (`rule:packaging/the-first-party-components-are-built-in`).
///
/// It has no `[[extension]]` entry. The digest `build.rs` took is its pin and its cache key, so
/// loading it hashes nothing. It has no `memory` ceiling of its own and no grants: the request's
/// memory cap bounds it, and the empty grant gives it no file and no host.
pub struct Builtin {
    path: PathBuf,
    bytes: &'static [u8],
    sha256: String,
    manifest: Manifest,
    source: Source,
    loader: Loader,
    extension: OnceLock<Result<Extension, Refused>>,
}

impl fmt::Debug for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builtin")
            .field("path", &self.path)
            .field("class", &self.manifest.class)
            .field("compiled", &self.extension.get().is_some())
            .finish_non_exhaustive()
    }
}

impl Builtin {
    /// The class it declares and that class's signatures, read without compiling.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The Novis source it carries.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// The digest of its bytes, taken when the binary was built: lower-case hexadecimal.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Whether [`Builtin::extension`] has run.
    pub fn compiled(&self) -> bool {
        self.extension.get().is_some()
    }

    /// The component as a loaded extension. The first call compiles it, or reads it from the
    /// loader's cache, and runs every check an `[[extension]]` entry passes but the reserved
    /// namespace. Every later call returns the same answer.
    ///
    /// # Errors
    ///
    /// Why the component does not load, which is a broken build.
    pub fn extension(&self) -> Result<&Extension, &Refused> {
        self.extension
            .get_or_init(|| {
                let component = self
                    .loader
                    .compile(&self.sha256, self.bytes)
                    .map_err(|err| {
                        refused(
                            &self.path,
                            format!("the file is not a valid component: {err:#}"),
                        )
                    })?;
                self.loader.check(
                    &self.path,
                    &component,
                    &self.manifest,
                    &self.source,
                    Reserve::Admit,
                )?;
                Ok(Extension {
                    path: self.path.clone(),
                    sha256: self.sha256.clone(),
                    memory: None,
                    grants: Granted::default(),
                    component,
                    manifest: self.manifest.clone(),
                    source: self.source.clone(),
                })
            })
            .as_ref()
    }
}

/// Every built-in component: its name, the bytes the binary carries and the digest the build took.
const BUILTINS: [(&str, &[u8], &[u8; 32]); 1] = [(
    "image",
    crate::builtin::IMAGE,
    &crate::builtin::IMAGE_SHA256,
)];

/// The manifests of the built-in components, read without an engine and without compiling: what
/// a compiler types a call into `Novis\` against, beside the manifests [`read_manifests`] reads.
/// Each carries its source section's files in [`Manifest::source`].
///
/// # Errors
///
/// The first built-in component whose sections do not read, which is a broken build.
pub fn builtin_manifests() -> Result<Vec<Manifest>, Refused> {
    BUILTINS
        .iter()
        .map(|&(name, bytes, _)| {
            let (mut manifest, source) =
                sections(&PathBuf::from(format!("built-in {name}.nvsx")), bytes)?;
            manifest.source = source.files;
            Ok(manifest)
        })
        .collect()
}

/// The manifests of the extensions `entries` name, read without compiling a component.
///
/// This is what a compiler reads (`rule:packaging/extension-calls-are-statically-typed`): the
/// checks of [`Loader::load`] the bytes answer alone — the pin, the two sections, a class outside
/// `Core\` and `Novis\`, a source file under the class's namespace — and [`Set::insert`]'s unique
/// class. Each manifest carries its source section's files in [`Manifest::source`]. What only the engine answers, the
/// component's validity, its imports and its exports, waits for the host that calls it.
///
/// # Errors
///
/// The index of the first entry that does not read, and why.
pub fn read_manifests(entries: &[Entry]) -> Result<Vec<Manifest>, (usize, Refused)> {
    read_manifests_with(entries, |entry| std::fs::read(&entry.path))
}

/// [`read_manifests`] over the bytes `read` returns for each entry, which a bundle answers from its
/// payload rather than from a file.
///
/// # Errors
///
/// As [`read_manifests`].
pub fn read_manifests_with(
    entries: &[Entry],
    read: impl Fn(&Entry) -> std::io::Result<Vec<u8>>,
) -> Result<Vec<Manifest>, (usize, Refused)> {
    let mut manifests: Vec<Manifest> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let refuse = |reason: String| (index, refused(&entry.path, reason));
        let bytes = read(entry).map_err(|err| refuse(format!("the file does not read: {err}")))?;
        let sha256 = pin(&bytes);
        if !sha256.eq_ignore_ascii_case(&entry.sha256) {
            return Err(refuse(format!(
                "the file's sha256 is {sha256}, and the entry pins {}",
                entry.sha256
            )));
        }
        let sections = section::read(&bytes).map_err(|err| refuse(err.0))?;
        let manifest = sections
            .manifest
            .ok_or_else(|| refuse("the component has no `nvs.manifest` section".to_owned()))?;
        let mut manifest = Manifest::parse(manifest).map_err(|err| refuse(err.0))?;
        let source = match sections.source {
            Some(source) => Source::parse(source).map_err(|err| refuse(err.0))?,
            None => Source {
                source: crate::source::FORMAT,
                files: Vec::new(),
            },
        };
        if let Some(reason) = reserved(&manifest) {
            return Err(refuse(reason));
        }
        if let Some(reason) = outside(&source, &manifest) {
            return Err(refuse(reason));
        }
        manifest.source = source.files;
        if let Some(other) = manifests
            .iter()
            .position(|other| other.class.eq_ignore_ascii_case(&manifest.class))
        {
            return Err(refuse(format!(
                "the class `{}` is already declared by the extension `{}`",
                manifest.class,
                entries[other].path.display()
            )));
        }
        manifests.push(manifest);
    }
    Ok(manifests)
}

/// Why `manifest`'s class may not load, when it is under `Core\` or `Novis\` in any case.
fn reserved(manifest: &Manifest) -> Option<String> {
    let namespace = manifest.class.split('\\').next().unwrap_or_default();
    ["Core", "Novis"]
        .iter()
        .any(|reserved| namespace.eq_ignore_ascii_case(reserved))
        .then(|| {
            format!(
                "the class `{}` is under `{namespace}\\`, which only Novis declares in",
                manifest.class
            )
        })
}

/// Why `source` may not load beside `manifest`, when one of its files declares a namespace outside
/// the class's own, the class minus its last segment ([`Source::outside`]).
fn outside(source: &Source, manifest: &Manifest) -> Option<String> {
    let namespace = manifest.class.rsplit_once('\\').map_or("", |(ns, _)| ns);
    source.outside(namespace).map(|(file, declared)| {
        let declares = declared.map_or_else(
            || "declares no namespace".to_owned(),
            |declared| format!("declares `namespace {declared}`"),
        );
        format!(
            "the source file `{}` {declares}, which is not under the extension's namespace `{namespace}`",
            file.path
        )
    })
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

/// Whether `func` is the WIT function `method`'s Novis signature gives, the named types resolved
/// against `manifest` and a resource against `resource`, the interface's export of that name.
fn check_signature(
    manifest: &Manifest,
    method: &Method,
    func: &ComponentFunc,
    resource: &dyn Fn(&str) -> Option<ResourceType>,
) -> Result<(), String> {
    let taken = Side {
        resource,
        returned: false,
    };
    let returned = Side {
        resource,
        returned: true,
    };
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
        let ty = manifest.novis_type(&param.ty)?;
        if !crosses_as(&ty, wit, &taken) {
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
        .then(|| manifest.novis_type(&method.returns))
        .transpose()?;
    let ok = match (&returns, result.ok()) {
        (None, None) => true,
        (Some(ty), Some(wit)) => crosses_as(ty, &wit, &returned),
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

/// Where a type crosses: the interface's resource of each name, and whether the type is a
/// result's, which returns a resource as an `own`, or a parameter's, which takes it as a `borrow`.
struct Side<'a> {
    resource: &'a dyn Fn(&str) -> Option<ResourceType>,
    returned: bool,
}

/// Whether a value of the Novis type `ty` crosses as `wit` on `side`.
fn crosses_as(ty: &NovisType, wit: &Type, side: &Side<'_>) -> bool {
    match (ty, wit) {
        (NovisType::Resource(name), Type::Own(wit)) if side.returned => {
            (side.resource)(name).is_some_and(|ty| ty == *wit)
        }
        (NovisType::Resource(name), Type::Borrow(wit)) if !side.returned => {
            (side.resource)(name).is_some_and(|ty| ty == *wit)
        }
        (NovisType::Bool, Type::Bool)
        | (NovisType::Int, Type::S64)
        | (NovisType::Uint, Type::U64)
        | (NovisType::Float, Type::Float64)
        | (NovisType::String, Type::String)
        | (NovisType::Mixed, Type::Borrow(_)) => true,
        (NovisType::Bytes, Type::List(list)) => matches!(list.ty(), Type::U8),
        (NovisType::List(item), Type::List(list)) => crosses_as(item, &list.ty(), side),
        (NovisType::Keyed(key, value), Type::List(list)) => match list.ty() {
            Type::Tuple(tuple) => {
                let types: Vec<Type> = tuple.types().collect();
                matches!(types.as_slice(), [k, v] if crosses_as(key, k, side) && crosses_as(value, v, side))
            }
            _ => false,
        },
        (NovisType::Optional(inner), Type::Option(option)) => crosses_as(inner, &option.ty(), side),
        (NovisType::Shape(fields), Type::Record(record)) => shape_crosses_as(fields, record, side),
        (NovisType::Enum { cases, .. }, Type::Enum(wit)) => {
            let wit: Vec<&str> = wit.names().collect();
            wit.len() == cases.len()
                && cases
                    .iter()
                    .zip(&wit)
                    .all(|(case, name)| crate::kebab(case) == *name)
        }
        (NovisType::Union { cases, .. }, Type::Variant(variant)) => {
            let wit: Vec<_> = variant.cases().collect();
            wit.len() == cases.len()
                && cases.iter().zip(&wit).all(|((name, fields), case)| {
                    case.name == crate::kebab(name)
                        && matches!(&case.ty, Some(Type::Record(record)) if shape_crosses_as(fields, record, side))
                })
        }
        (NovisType::Core(core), Type::Record(record)) => {
            let wit: Vec<_> = record.fields().collect();
            wit.len() == core.fields.len()
                && core.fields.iter().zip(&wit).all(|((name, ty), field)| {
                    field.name == *name && crosses_as(ty, &field.ty, side)
                })
        }
        _ => false,
    }
}

/// Whether a shape of `fields` crosses as `record` on `side`: its keys in kebab-case, in order,
/// and an optional one an `option`.
fn shape_crosses_as(fields: &[Field], record: &Record, side: &Side<'_>) -> bool {
    let wit: Vec<_> = record.fields().collect();
    wit.len() == fields.len()
        && fields
            .iter()
            .zip(&wit)
            .all(|((name, optional, ty), field)| {
                field.name == crate::kebab(name)
                    && if *optional {
                        matches!(&field.ty, Type::Option(option) if crosses_as(ty, &option.ty(), side))
                    } else {
                        crosses_as(ty, &field.ty, side)
                    }
            })
}
