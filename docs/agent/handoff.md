# Handoff

## State

Goal `core-cli-progress-and-6-more` is complete. Its two red acceptance checks — `dossier:
Core\Compress\Compressor` and `dossier: Core\Compress\Decompressor` — now both report `nothing
owed` with `0 failed` twice. `Core\Compress\Compressor::add` and `::finish` and
`Core\Compress\Decompressor::add` and `::finish` each carry an `about.md`, three blessed examples,
one attack, one recorded bench figure and a Rust `#[test]` with its `covers:` marker.

The three gates a goal meets only at its end are clean: `verify.py --doc` resolves every link, and
`owners.py --closes` and `playbook.py --closes` both report that this goal owns nothing.

No proof found a bug. Two figures are worth keeping. Both `add` members cost **0 allocations per
call**, which their benches now declare, so the retention in `add_chunk`
(`crates/nvs-stdlib/src/compress.rs:1140`) is a guarded number rather than a doc comment; the Rust
test beside it pins the same claim as a refcount. `Compressor::finish` costs 162 allocations and
288 KB per call against `Decompressor::finish`'s 52 and 12 KB, which is the per-call compression
context already in `## Backlog`.

The pack's `[context] modules` names only `nvs-stdlib`'s three files. A Rust proof that asserts a
refcount also needs `crates/nvs-runtime/src/string.rs` (`NvsStr::refcount_of`) and
`crates/nvs-runtime/src/value.rs` (`Value::buffer_ptr`); both had to be grepped for.

## Next group

**Goal `core-config` — `Core\Config`** — one file set: `crates/nvs-stdlib/src/config.rs`,
`docs/examples/core/Config/`, `tests/hostile/core/Config/`, `benches/members/core/Config/`. All
three are `rule:testing/feature-proofs`, and `Core\Compress`'s eight landed members are the model
for what each proof looks like.

- [ ] **`Core\Config::all`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/config.rs:66`
- [ ] **`Core\Config::get`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/config.rs:39`
- [ ] **`Core\Config::restore`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/config.rs:57`

## Backlog

- A compression context is built per call rather than per stream: `Core\Compress::compress` costs
  130 allocations and 280 KB on a 45-byte record and `Core\Compress\Compressor::finish` 162 and
  288 KB, against 23/9 KB and 52/12 KB for the two decompressing halves. `docs/perf/members.ndjson`.
