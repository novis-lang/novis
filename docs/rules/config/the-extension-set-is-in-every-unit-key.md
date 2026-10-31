```
extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash ‖ regex_backtracking)
content_hash       = BLAKE3(source_content)
artifact_key       = BLAKE3(content_hash ‖ env_hash)
```

One `env_hash` is carried by **both** compiled-unit caches: the on-disk key is
`BLAKE3(content_hash ‖ env_hash)`, and the in-memory key is `UnitKey { path, content_hash,
env_hash }`. It is derived from the same `content_hash` the in-memory key carries, so a unit's bytes
are hashed once for both. It is constant for the life of a configuration and costs the request path
nothing.

A manifest lives inside its `.nvsx` as a custom section, so hashing the pins covers each extension's
declared classes, signatures and qualifier declarations with no separate manifest hash. **A class
name declared by two extensions is refused at load**, so the set is order-independent and the sorted
hash is well defined. An entry with no pin contributes its path, so it still rekeys every unit.

`regex_backtracking` is `[regex] backtracking`, one byte, the one other directive that decides what
a unit compiles to: under `warn` or `deny` a backtracking pattern literal warns or does not compile
(`rule:core-classes/regex-compile-time-tiering`). A reload that changes it rekeys every unit, as a
changed extension set does.

`compiler_version_hash` is **the running compiler executable, not the release version**: its path, its
byte length and its modification time, read once per process. A version string is one string for every
build of an unreleased tree, so keying on it alone would let a rebuilt compiler read back artifacts its
predecessor emitted — correctly checksummed bytes of this same version, emitted by different codegen.
A compiler that cannot examine its own binary keys apart from every one that can. The binary's contents
would be the exact identity and are not read: a pass over tens of megabytes at every process start buys
only the case where something can rewrite the compiler in place, which is already the compiler.

That is the whole of extension reload: a changed set changes `env_hash`, every unit key changes with
it, every lookup is an ordinary miss, and the lazy per-path revalidation of
`rule:config/an-edit-reaches-the-next-request-without-a-restart` recompiles each unit on the compile
pool as some request resolves it. No invalidation pass exists. It also closes the hole where an
artifact compiled against one extension set — holding a direct call to a trampoline that has since
moved — could be reused against another.

Invalidation is coarse by design: changing the set rekeys every unit, not only units that call an
extension. The first request to resolve each path afterwards pays a compile, on the compile pool and
not on a request-serving core, and the pool bounds how much of the wave is in flight at once. That
is still strictly cheaper than the restart it replaces, which pays the same compiles cold and drops
every in-flight request.

- **Memory:** an extension change keeps one extra generation of compiled units, each held exactly
  as long as some in-flight request holds it. O(in-flight), never O(reloads performed).

**Declined:**
- **Invalidating only the units that call a changed extension.** It needs per-unit dependency
  tracking including negative dependencies, since a unit that failed because a class was unknown
  must retry when the class appears: a real subsystem for a modest win. If the wave ever measures as
  a real cost, the answer is precompiling entry points before the swap, not that tracking.
