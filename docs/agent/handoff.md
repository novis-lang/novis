# Handoff

## State

Goal `core-cli-progress-and-6-more`: twelve of its sixteen features are done. `Core\Compress`'s four
members — `compress`, `decompress`, `compressor` and `decompressor` — each carry an `about.md`, three
examples with blessed `.out` files, one attack, one bench with a recorded figure, and a Rust `#[test]`
with its `covers:` marker. `python tools/dossier.py --group 'Core\Compress'` shows every column filled.

No proof found a bug. The first measurement did surface one thing worth a look: `Core\Compress::compress`
costs 130 allocations and 280 KB per call on a 45-byte record, against 23 and 9 KB for `decompress`,
which is a compression context built per call rather than per stream. It is in `## Backlog`.

Nothing is blocked. The four remaining features are the two stream classes' members, all in
`compress.rs`, and `python tools/dossier.py --group 'Core\Compress\Compressor'` names what each owes.

## Next group

**The two stream classes' four members** — one file set, the same one the landed four used:
`crates/nvs-stdlib/src/compress.rs`, `docs/examples/core/Compress/`, `tests/hostile/core/Compress/`,
`benches/members/core/Compress/`. All four are `rule:testing/feature-proofs`, and the four landed
members are the model for what each proof looks like. Both classes' members are **instance** members,
so `rule:testing/proof-attribution` gives a `covers:` marker as the only thing attributing a test —
a `.nvst` case that calls one is not credited by the call the way a `Core\Compress::` member is.

- [ ] **`Core\Compress\Compressor::add`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/compress.rs:1284`. Its subject is what a chunk costs: `add_chunk`
      retains the argument rather than copying it, so feeding one buffer a thousand times is one
      buffer, which the compressor attack already shows.
- [ ] **`Core\Compress\Compressor::finish`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/compress.rs:1293`. It is where the whole concatenation is materialised,
      and where a stream fed more than the request may hold meets the memory limit.
- [ ] **`Core\Compress\Decompressor::add`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/compress.rs:1303`. It takes `bytes` and no `string`, unlike the
      compressing half's `add`.
- [ ] **`Core\Compress\Decompressor::finish`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/compress.rs:1316`. The bound is measured here, once over every piece,
      which `rule:core-classes/decompression-bound` is the home of.

## Backlog

- `Core\Compress::compress` allocates 130 times and 280 KB per call on a 45-byte record
  (`docs/perf/members.ndjson`); a context held per stream rather than per call is the shape to price.
- `Core\Compress\Compressor::finish` joins every chunk into one buffer, so a stream costs the total
  fed rather than a flat window — `crates/nvs-stdlib/src/compress.rs`'s module doc prices it already.
