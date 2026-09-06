```
extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
content_hash       = BLAKE3(source_content)
artifact_key       = BLAKE3(content_hash ‖ env_hash)
```

One `env_hash` is carried by **both** compiled-unit caches: the on-disk key is
`BLAKE3(content_hash ‖ env_hash)`, and the in-memory key is `UnitKey { path, content_hash,
env_hash }`. It is derived from the same `content_hash` the in-memory key carries, so a unit's bytes
are hashed once for both. It is constant for the life of a configuration and costs the request path
nothing.

That is the whole of extension reload: a changed set changes `env_hash`, every unit key changes with
it, every lookup is an ordinary miss, and the lazy per-path revalidation of
`rule:config/an-edit-reaches-the-next-request-without-a-restart` recompiles each unit on the compile
pool as some request resolves it. No invalidation pass exists. It also closes the hole where an
artifact compiled against one extension set — holding a direct call to a trampoline that has since
moved — could be reused against another.

Invalidation is coarse by design: changing the set rekeys every unit, not only units that call an
extension. Finer would need per-unit dependency tracking including negative dependencies, a real
subsystem for a modest win. The compile pool bounds how much of the resulting wave is in flight at
once.
