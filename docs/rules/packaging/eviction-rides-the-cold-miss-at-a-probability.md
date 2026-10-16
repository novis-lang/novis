A content-addressed entry never needs invalidating for correctness, only for growth: the key covers
every input to a compile, so changing any of them produces ordinary misses and no invalidation pass
exists. Eviction is therefore purely a size question, and it hangs off the **miss**. After a store —
already the expensive path, on the compile pool — with a small configured probability, the writer walks
the cache directory's total size and, if it is over the configured cap, deletes oldest-by-`mtime`
entries down to a hysteresis floor below the cap, so a cache hovering at the boundary does not walk on
every subsequent miss.

The probability is `opcache.file_cache_gc_probability` over `opcache.file_cache_gc_divisor`; the cap is
`opcache.file_cache_max_size`. **A warm hit never performs a directory walk, never checks a size, and
pays nothing beyond verify-then-map.**

The cache may transiently sit above its cap between the misses that happen to roll a sweep. That is the
accepted trade, bounded by how unlikely a long silent stretch of pure hits is, and correctable at any
time through `rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`.
