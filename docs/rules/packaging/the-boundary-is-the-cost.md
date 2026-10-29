Measured in a release build: a host-to-guest call costs about 11.5 ns, a guest-to-host accessor call
about 9 ns, a 1 KiB bulk copy into guest memory about 12 ns, and a fresh pooled instance plus one call
about 8 µs. A built-in call frame costs under a nanosecond, so **an extension call carries roughly 10 ns
more overhead than a built-in one**.

In-guest compute is measured against the same crate built native in
`benches/results/image-guest.json`, which `crates/nvs-ext/benches/image_guest.rs` writes and
`crates/nvs-ext/tests/image_guest.rs` holds to byte-identical output on both arms. On a full HD frame,
a whole call that returns the pixels takes about twice as long in the guest as native, and returning
the pixels adds little to returning the size header alone. That holds because the image world is
called typed (`crates/nvs-ext/src/typed.rs`), so its byte lists cross in one copy each way, as every
built-in world's do. A third-party export is called on the dynamic path, which would cross a byte
list as one wasmtime `Val` per byte. `crates/nvs-ext/benches/ext_bytes.rs` measured that in
`benches/results/ext-bytes.json`: a 16 MiB echo took 876 ms and allocated 122 host bytes per byte
crossed. So a third-party `bytes` crosses as the host's `bytes` resource instead, copied once each
way by typed host calls (`rule:packaging/a-value-crosses-as-its-wit-type`).

That is noise for coarse-grained work — image codecs, compression, crypto, document parsing — and
decisive for fine-grained work, which is why primitives are Tier 0 and performance-critical first-party
subsystems are Tier 2 (`rule:core-api/five-placements`). An extension author controls the boundary, not the
compute, so an extension's API is designed **coarse**: whole inputs in, whole outputs out, a batch where
a library would usually offer a per-item call. Collation exposes sort-key generation and whole-array sort rather than
a comparator, because sorting ten thousand strings through a per-comparison boundary would be about
130,000 crossings; the image component crosses once per terminal
(`rule:core-classes/image-pipeline`).

A capability whose in-guest compute is too slow for a real workload moves to Tier 0 or Tier 2. It never
opens a native loading path (`rule:security/no-ffi`).
