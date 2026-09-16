# Handoff

## State

**Goal `unowned-closures`, stage 4 open.** `Core\Compress`'s incremental surface is on disk and its
gap is struck. `Core\Compress::compressor` and `::decompressor` open two classes — a
`Compress\Compressor`, whose `finish` answers `bytes`, and a `Compress\Decompressor`, whose `finish`
answers `tainted bytes` — each fed by `add` and closed by `finish`. A decompressor resolves `Bound`
at the opening and `finish` applies it once over the whole stream, so feeding a frame in ten chunks
buys exactly what feeding it in one does. `crates/nvs-stdlib/src/compress.rs:45` § *The incremental
surface, and why it is two classes* is the home of why two classes, why the chunks rather than a
coder, why PHP's `$flush_mode` has no spelling, and what a stream spends.

**The failing acceptance check is not what this group closed.** Stage 4's check is `python
tools/owners.py` wanting `, 0 owned by a retired goal`, and the five items it names belong to retired
goal `m8-stdlib-depth` — not one of them is a `Core\Compress` gap. They are the group below, which is
a different file set and needs its own pack. Nothing is blocked.

## Next group

**Stage 4: the module docs — the five gaps retired goal `m8-stdlib-depth` left** — one file set:
`crates/nvs-stdlib/src/debug.rs`, `crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/db/mod.rs`
and `crates/nvs-render/src/lib.rs`. `python tools/owners.py` prints the list; the verdict for each is
*build it and strike the gap*, or re-owner to an M9+ milestone whose plan states the scope. `unowned`
is not available — this goal exists to empty it.

- [ ] **`[debug] inline` is the channel a dump reaches a response body through** —
      `crates/nvs-stdlib/src/debug.rs:35` gap 1 and `crates/nvs-stdlib/src/debug.rs:43` gap 2, one
      file: the directive and the record beside it, then `render`'s two carriers
      (`rule:errors/debug-dump`, `rule:errors/renderings`). Gap 2 names its own precondition — a
      `Core` class cannot be narrowed by `instanceof`, which is `E0496` — so read the gap before
      building rather than after.
- [ ] **An `array<T>` of inline shapes needs an element description that nests** —
      `crates/nvs-stdlib/src/json.rs:151` gap 1 (`rule:core-classes/derive-field-list`): the
      element's contract has nowhere to ride, since `nvs_runtime::CodecField` carries one and a list
      has spent it naming the element's wire type. The same widening `array<array<T>>` waits on, so
      this is the one of the five most likely to be a milestone owner rather than a build.
- [ ] **A hydration's skipped field has no call site to emit its default from** —
      `crates/nvs-stdlib/src/db/mod.rs:268` gap 3 (`rule:core-classes/derive-field-list`), the
      neighbouring question at the other door to the `json.rs` one; `nvs_types::derive`'s
      `check_row_sites` already refuses every other shape as `E0806` while compiling.
- [ ] **A `#[Test]` result is a producer, so § 22's three output formats are one record rendered
      three ways** — `crates/nvs-render/src/lib.rs:39` gap 1 (`rule:errors/diagnostic-record`), the
      only one of the five outside `nvs-stdlib`.

## Backlog

- The `[context] modules` manifest names `crates/nvs-stdlib/src/compress.rs` but not `hash.rs`,
  `instance.rs`, `registry.rs` or `identity_store.rs` — a `Core` instance class cannot be written
  without all four, so add them.
- `[context] rules` is missing `core-api/shape-rules`, `core-api/verb-lexicon`,
  `core-api/symmetric-names` and `security/tainted-sources`; naming a new `Core` class needs them and
  the last one decided this group's design.
- `[context]` has no way to name a test file, so `crates/nvs-stdlib/tests/conformance_coverage.rs`
  and `tests/corpus/mod.rs` — the floor of three and how a case is attributed to a class — were read
  from scratch; `modules` takes only crate `src/` paths.
- The four remaining `past-milestone` findings `python tools/owners.py` prints (M1, M6, M7 owners on
  live gaps) are not the stage-4 check but are the same kind of work, one call away.
- `python tools/gaps.py --coverage` ranks the classes still nearest the floor of three.
