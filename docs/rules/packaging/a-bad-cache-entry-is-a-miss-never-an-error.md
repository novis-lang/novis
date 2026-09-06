No failure on the cache path reaches a panic, a `FATAL` or a `Throwable`. A wrong magic, format
version or environment, a truncated payload, a checksum that does not match, a symbol this process
cannot resolve, a directory that cannot be written — every one of them is a cache miss, and the script
being run sees exactly what it would see with a cold cache. This is `rule:errors/propagation`'s
checked-return discipline extended to a compile-pipeline internal that never had a caller to report to.

Which failures delete the file is decided by what the failure means. A checksum mismatch, or a
`payload_len` that disagrees with the mapping, deletes it: the key is the content hash, so a file at that
path whose bytes hash differently can only be corrupt or tampered, never a second valid version, and
leaving it costs a re-open on every future run. A wrong magic, `format_version` or `env_hash`, and an
unresolvable symbol, do **not**: those say "not this process's file" rather than "broken", and deleting
on them would let one build of the compiler evict another's entries out of a shared directory.

Losing an un-synced entry to a crash is the same shape: the next process recompiles. The one thing that
is not a silent miss is a cache directory another account can write, which
`rule:packaging/the-checksum-proves-integrity-and-ownership-proves-trust` refuses out loud, because that
is a breach of the boundary and not a bad entry.
