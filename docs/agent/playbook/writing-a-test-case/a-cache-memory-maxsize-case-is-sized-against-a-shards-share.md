- **A `[cache.memory] max_size` case is sized against a shard's *share*: the tier is 64 shards and each
  evicts against `max_size / 64`.** A cap whose share is under `ENTRY_OVERHEAD` forgets every entry as
  it arrives, so the case asserts nothing. Keep an entry under the share, cross the whole cap several
  times over, and assert the bound rather than which key went, because which shard a key lands in is a
  hash's business. See `crates/nvs-stdlib/src/cache.rs`.
  `until: gone crates/nvs-stdlib/src/cache.rs:ENTRY_OVERHEAD`
