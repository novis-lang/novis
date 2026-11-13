//! `rule:packaging/the-first-party-components-are-built-in`'s stored form: the binary carries each
//! built-in component compressed with Brotli and its two sections plain. A component expands to
//! the bytes its pin names, the stored form is under half the expanded size, a manifest reads
//! without expanding anything, and a component the cache holds loads without expanding.
//!
//! The expansion count is a debug-build count per thread, and a component is compiled on the
//! thread that asks for it, so a test beside these adds nothing to it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nvs_ext::builtin::{STORED, expanded, expansions};
use nvs_ext::load::{CacheKey, Loader, ModuleCache, builtin_manifests, pin};
use wasmtime::Engine;

/// A cache in memory.
#[derive(Default)]
struct Memory(Mutex<HashMap<CacheKey, Vec<u8>>>);

impl ModuleCache for Memory {
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        self.0.lock().unwrap().get(key).cloned()
    }

    fn put(&self, key: &CacheKey, bytes: &[u8]) {
        self.0.lock().unwrap().insert(key.clone(), bytes.to_vec());
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn every_built_in_component_expands_to_the_bytes_its_pin_names() {
    for stored in STORED {
        let bytes = expanded(stored.name)
            .unwrap_or_else(|err| panic!("`{}` expands: {}", stored.name, err.0));
        assert_eq!(pin(&bytes), hex(&stored.sha256), "`{}`", stored.name);
        let sections = nvs_ext::section::read(&bytes).expect("the expanded bytes are a component");
        assert_eq!(
            sections.manifest,
            Some(stored.manifest),
            "`{}`",
            stored.name
        );
        assert_eq!(
            sections.source.unwrap_or_default(),
            stored.source,
            "`{}`",
            stored.name
        );
    }
    assert!(expanded("nothing").is_err(), "an unknown name expands");
}

#[test]
fn every_built_in_component_is_stored_at_under_half_its_size() {
    let mut stored_total = 0;
    let mut expanded_total = 0;
    for stored in STORED {
        let size = expanded(stored.name).expect("the component expands").len();
        assert!(
            stored.compressed.len() * 2 < size,
            "`{}` is stored in {} bytes of {size}",
            stored.name,
            stored.compressed.len()
        );
        stored_total += stored.compressed.len();
        expanded_total += size;
    }
    assert!(stored_total * 2 < expanded_total);
}

#[test]
fn a_built_in_manifest_reads_without_expanding_its_component() {
    let before = expansions();
    let manifests = builtin_manifests().expect("the built-in manifests read");
    assert_eq!(manifests.len(), STORED.len());
    let builtins = Loader::new(&Engine::default())
        .builtins()
        .expect("the built-in components load");
    for (builtin, stored) in builtins.iter().zip(STORED) {
        assert_eq!(builtin.manifest().class, stored.class);
        assert!(!builtin.source().files.is_empty() || stored.source.is_empty());
        assert!(!builtin.compiled());
    }
    assert_eq!(
        expansions(),
        before,
        "reading a manifest expanded a component"
    );
}

#[test]
fn a_cached_built_in_component_loads_without_expanding() {
    let cache = Arc::new(Memory::default());
    let engine = Engine::default();
    let spreadsheet = |loader: &Loader| {
        loader
            .builtins()
            .expect("the built-in components load")
            .into_iter()
            .find(|builtin| builtin.name() == "spreadsheet")
            .expect("the spreadsheet component is built in")
    };
    let before = expansions();
    let cold = spreadsheet(&Loader::new(&engine).with_cache(cache.clone()));
    cold.extension()
        .expect("the spreadsheet component compiles");
    assert_eq!(
        expansions(),
        before + 1,
        "a miss expands the component once"
    );
    let warm = spreadsheet(&Loader::new(&engine).with_cache(cache));
    warm.extension()
        .expect("the spreadsheet component loads from the cache");
    assert_eq!(
        expansions(),
        before + 1,
        "a cache hit expanded the component"
    );
}
