The on-disk artifact cache is a fan-out directory of immutable files, one per compiled unit:

```
<cache_dir>/<key[0:2]>/<key[2:]>.nvsc
```

where `key = BLAKE3(content_hash ‖ env_hash)` and `content_hash` is BLAKE3 over every file the program
reached, each as its name, its text and its folder. The name is there because a diagnostic and a
throw's frame print it. The folder is there because a relative path given as a string literal compiles to an absolute
path joined to it (`rule:programs/relative-paths-resolve-from-their-file`), so the same file in another
folder is another program. The content hash is computed once per unit and shared with the in-memory
`UnitKey`, so a unit's bytes cross BLAKE3 one time however many caches it lands in. `env_hash` is the single environment digest of
`rule:config/the-extension-set-is-in-every-unit-key` — target triple, CPU feature bitset, compiler
build and the loaded extension set — and the same value keys the in-memory cache, so one process
cannot disagree with its own disk cache about what a unit was compiled against.

**The environment is in the address, not only in the header.** An artifact built for another machine,
another CPU-feature set, another compiler build or another extension set is a path this process never
looks up; the cost is one failed `open`, the same as any miss, never an open-then-reject. The extension
set belongs in the key because codegen emits a direct call to an extension trampoline, so the loaded
set is a codegen input like any other.

Both digests are cryptographic on purpose. One `nvs serve` shares one `env_hash` across every tenant's
source, so a key a tenant could collide on purpose would hand their artifact to another tenant's
request, and `rule:packaging/a-writer-publishes-by-one-atomic-rename-and-never-a-lock` is true only of a
collision-resistant key. No shared index, no lock file and no manifest exist anywhere: every entry is
independently creatable, verifiable and discardable, so a bug or an attack against one entry has a
blast radius of exactly one entry. The `[opcache]` directives that govern the store are
`rule:config/opcache-file-cache-directives-are-system`'s.
