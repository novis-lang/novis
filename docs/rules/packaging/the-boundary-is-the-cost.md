Measured in a release build: a host-to-guest call costs about 11.5 ns, a guest-to-host accessor call
about 9 ns, a 1 KiB bulk copy into guest memory about 12 ns, and a fresh pooled instance plus one call
about 8 µs. A built-in call frame costs under a nanosecond, so **an extension call carries roughly 10 ns
more overhead than a built-in one**. In-guest compute throughput relative to native is not yet measured
and is not claimed.

That is noise for coarse-grained work — image codecs, compression, crypto, document parsing — and
decisive for fine-grained work, which is why primitives are Tier 0 and performance-critical first-party
subsystems are Tier 2 (`rule:packaging/three-tiers`). An extension author controls the boundary, not the
compute, so an extension's API is designed **coarse**: whole inputs in, whole outputs out, a batch where
a library would usually offer a per-item call. Collation exposes sort-key generation and whole-array sort rather than
a comparator, because sorting ten thousand strings through a per-comparison boundary would be about
130,000 crossings; the image component crosses once per terminal
(`rule:core-classes/image-pipeline`).
