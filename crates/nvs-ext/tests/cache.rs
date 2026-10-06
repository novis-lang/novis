//! `rule:packaging/a-wasm-module-cache-reuses-the-artifact-cache` as `nvs-ext` sees it: a compiled
//! component is stored under the file's pin and the engine's environment, loaded back instead of
//! compiled under both, and a bad entry is a miss
//! (`rule:packaging/a-bad-cache-entry-is-a-miss-never-an-error`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nvs_ext::load::{CacheKey, Entry, Loader, ModuleCache, pin};
use nvs_ext::pack::append_section;
use nvs_ext::section::MANIFEST;
use wasmtime::{Config, Engine};

/// A component exporting `shop:geo/api` with one function,
/// `distance-km: func(from: string) -> result<f64, error>`.
fn guest() -> Vec<u8> {
    let component = wat::parse_str(
        r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (core module $m
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
    (func (export "distance-km") (param i32 i32) (result i32) i32.const 16))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "realloc" (core func $realloc))
  (func $f (param "from" string) (result (result f64 (error $error)))
    (canon lift (core func $i "distance-km") (memory $mem) (realloc $realloc)))
  (instance $api (export "distance-km" (func $f)))
  (export "shop:geo/api" (instance $api)))"#,
    )
    .expect("the test component compiles");
    let manifest = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Geo", "interface": "shop:geo/api", "methods": [{"name": "distanceKm", "params": [{"name": "from", "type": "string"}], "returns": "float"}]}"#;
    append_section(component, MANIFEST, manifest.as_bytes())
}

/// A cache in memory, counting what the loader stores.
#[derive(Default)]
struct Memory {
    entries: Mutex<HashMap<CacheKey, Vec<u8>>>,
    puts: Mutex<usize>,
}

impl ModuleCache for Memory {
    fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        self.entries.lock().unwrap().get(key).cloned()
    }

    fn put(&self, key: &CacheKey, bytes: &[u8]) {
        *self.puts.lock().unwrap() += 1;
        self.entries
            .lock()
            .unwrap()
            .insert(key.clone(), bytes.to_vec());
    }
}

impl Memory {
    fn puts(&self) -> usize {
        *self.puts.lock().unwrap()
    }

    fn keys(&self) -> Vec<CacheKey> {
        self.entries.lock().unwrap().keys().cloned().collect()
    }
}

fn loader(engine: &Engine, cache: &Arc<Memory>) -> Loader {
    Loader::new(engine).with_cache(cache.clone())
}

/// Loads `bytes` under its own pin, and checks the extension is the one the file declares.
fn load(loader: &Loader, bytes: &[u8]) {
    let entry = Entry {
        path: PathBuf::from("geo.nvsx"),
        sha256: pin(bytes),
        memory: None,
    };
    let extension = loader
        .load_bytes(&entry, bytes)
        .expect("the extension loads");
    assert_eq!(extension.manifest.class, "Shop\\Geo");
}

#[test]
fn a_compiled_component_is_stored_under_its_digest_and_the_environment() {
    let bytes = guest();
    let cache = Arc::new(Memory::default());
    let loader = loader(&Engine::default(), &cache);
    load(&loader, &bytes);
    assert_eq!(
        cache.keys(),
        [CacheKey {
            pin: pin(&bytes),
            environment: loader.environment().to_owned(),
        }]
    );
    assert_eq!(loader.environment().len(), 64);
    assert_eq!(
        loader.environment(),
        Loader::new(&Engine::default()).environment(),
        "two engines of one configuration share an environment"
    );
}

#[test]
fn a_stored_compiled_component_is_loaded_without_compiling_again() {
    let bytes = guest();
    let cache = Arc::new(Memory::default());
    load(&loader(&Engine::default(), &cache), &bytes);
    assert_eq!(cache.puts(), 1);
    load(&loader(&Engine::default(), &cache), &bytes);
    assert_eq!(
        cache.puts(),
        1,
        "the second load stored nothing, so it compiled nothing"
    );
}

#[test]
fn a_compiled_component_stored_under_another_environment_is_never_loaded() {
    let bytes = guest();
    let cache = Arc::new(Memory::default());
    let first = loader(&Engine::default(), &cache);
    load(&first, &bytes);
    let mut config = Config::new();
    config.consume_fuel(true);
    let other = loader(&Engine::new(&config).expect("the engine builds"), &cache);
    assert_ne!(first.environment(), other.environment());
    load(&other, &bytes);
    assert_eq!(cache.puts(), 2, "the other engine compiled its own");
    let mut environments: Vec<String> = cache
        .keys()
        .into_iter()
        .map(|key| key.environment)
        .collect();
    environments.sort();
    let mut expected = vec![
        first.environment().to_owned(),
        other.environment().to_owned(),
    ];
    expected.sort();
    assert_eq!(environments, expected);
}

#[test]
fn a_corrupt_stored_compiled_component_is_a_miss_and_is_compiled_again() {
    let bytes = guest();
    let cache = Arc::new(Memory::default());
    let loader = loader(&Engine::default(), &cache);
    load(&loader, &bytes);
    let key = cache.keys().remove(0);
    let corruptions: [fn(&mut Vec<u8>); 3] = [
        |stored| {
            let last = stored.len() - 1;
            stored[last] ^= 0xff;
        },
        |stored| stored.truncate(stored.len() / 2),
        |stored| stored.truncate(5),
    ];
    for (puts, corrupt) in (2..).zip(corruptions) {
        corrupt(cache.entries.lock().unwrap().get_mut(&key).unwrap());
        load(&loader, &bytes);
        assert_eq!(cache.puts(), puts, "the corrupt entry was compiled again");
        load(&loader, &bytes);
        assert_eq!(cache.puts(), puts, "the entry stored in its place is a hit");
    }
}
