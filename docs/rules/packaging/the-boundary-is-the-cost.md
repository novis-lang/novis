Measured in a release build: a host-to-guest call costs about 11.5 ns, a guest-to-host accessor call
about 9 ns, a 1 KiB bulk copy into guest memory about 12 ns, and a fresh pooled instance plus one call
about 8 µs. A built-in call frame costs under a nanosecond, so **an extension call carries roughly 10 ns
more overhead than a built-in one**.

In-guest compute is measured against the same crate built native in
`benches/results/image-guest.json`, which `crates/nvs-ext/benches/image_guest.rs` writes and
`crates/nvs-ext/tests/image_guest.rs` holds to byte-identical output on both arms. A JPEG decode in the
guest takes two to three times as long as native when only its size header comes back. A whole call
that returns the pixels takes over twenty times as long, for a resize as for a decode, because a byte list
crosses as one wasmtime `Val` per byte
(`data/gaps/nvs-ext/bytes-cross-one-val-per-byte.json`). So today the copy, not the compute, is
what a large image pays.

That is noise for coarse-grained work — image codecs, compression, crypto, document parsing — and
decisive for fine-grained work, which is why primitives are Tier 0 and performance-critical first-party
subsystems are Tier 2 (`rule:packaging/three-tiers`). An extension author controls the boundary, not the
compute, so an extension's API is designed **coarse**: whole inputs in, whole outputs out, a batch where
a library would usually offer a per-item call. Collation exposes sort-key generation and whole-array sort rather than
a comparator, because sorting ten thousand strings through a per-comparison boundary would be about
130,000 crossings; the image component crosses once per terminal
(`rule:core-classes/image-pipeline`).
