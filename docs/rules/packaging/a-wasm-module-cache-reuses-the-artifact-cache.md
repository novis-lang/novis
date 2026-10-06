A compiled extension module is cached in the same store as a compiled unit, not in a second cache of
its own. Nothing in `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` is specific to
native code: a payload is a payload, and the same content-addressed key, the same self-describing
header, the same verify-before-map read path and the same piggybacked eviction apply, with the module
runtime's own serialization versioning standing in for the compiler-version component of `env_hash`.

The rule is that the cache is one mechanism with one trust story. An extension's compiled form is
subject to the same ownership check on the directory and the same checksum on the file as a program's,
and a wrong-environment module is a miss at the path rather than a file that is opened and rejected.

**Not on disk.** `nvs_ext::load::Loader` stores a compiled component behind a `ModuleCache` seam,
keyed by the file's pin and a digest of wasmtime's compatibility hash, and a checksum mismatch or an
entry wasmtime does not accept is a miss that is compiled and stored again
(`crates/nvs-ext/tests/cache.rs`). Nothing implements the seam over `nvs-cli`'s artifact store yet.
