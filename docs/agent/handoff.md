# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`crates/nvs-stdlib/src/random.rs` has no `# Known gaps` section left: gap 1 is built, so `python
tools/owners.py --closes decided-closures` names one fewer gap than it did. `zip.rs` (session 0001)
and `path.rs` (session 0002) were cleared the same way.

`Core\Random\Seeded` is registered: spec § 11's reproducible generator as an instance class with a
`registry::CONSTRUCTORS` entry, so `new Core\Random\Seeded(42)` resolves and holds a SplitMix64
state in one `int` slot. Its seven members mirror `Core\Random`'s, each reading that slot, drawing,
and writing the advanced state back **after** the draw — so a refused call leaves the sequence where
it found it. The draws themselves are now functions over `Generator` that both classes call, while
each member keeps its own argument decoding and its own refusal sentences: a message names the class
it came from, and `conformance_coverage.rs`'s error-path gate reads a literal at the `Fault::` site
rather than a format hole.

**Stage 4's three acceptance checks are still red, and all of them for the first reason** — the
member does not exist yet. Of the first check's six tests only
`a_uuid_round_trips_through_its_sixteen_bytes` is still missing, which is the group below.

## Next group

**Stage 4: `Core\Uuid`'s bytes pair, and the spec amendment that admits it** — one file set:
`crates/nvs-stdlib/src/uuid.rs`, its `.nvst` cases under `tests/conformance/core/`, and § 11's
second table in `docs/spec/01-core-library.md`. No rule owns the pair; the gap's own `Decided:`
sentence is the specification, and `docs/agent/loop-goal.md` § *Standing decisions* names it as one
of the three answers that differ from the sheet's recommendation.

- [ ] **`crates/nvs-stdlib/src/uuid.rs:134` — add the bytes pair: `Core\Uuid::fromBytes(bytes $b)`
      as a static member answering `CoreTy::Instance(NAME)`, and `toBytes(): bytes` as an instance
      member beside `toString`.** The five edits are `docs/agent/conventions.md` § *A `Core`
      member*; the class already builds instances, so the slot layout at
      `crates/nvs-stdlib/src/uuid.rs:134`'s `CLASS` is what both read, and
      `crates/nvs-stdlib/src/random.rs:105` is this goal's own worked example of an instance member
      pair landing together. Sixteen bytes exactly, and a `bytes` value of any other length is an
      R4 throw naming the length it got. The acceptance name is
      `a_uuid_round_trips_through_its_sixteen_bytes`. Delete the numbered gap at
      `crates/nvs-stdlib/src/uuid.rs:102` when it lands, and say what the pair spends in the module
      doc's `# What it spends` (`rule:programs/memory-priority`) — it is one `NvsStr` per call.
- [ ] **`docs/spec/01-core-library.md:789` — § 11's second table gains the two rows.** The standing
      decision calls this a spec amendment, so the table is the one that moves rather than the
      registry mirroring something nobody wrote.
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs`'s
      `every_registry_rows_names_are_the_specs_signature_column` compares the Signature cell's
      `$names` against the row's `names` and fails on a drift, so write the cell and the row in one
      slice.

## Backlog

- The rest of stage 4, after the pair: `queue.rs`'s fifth counter and boot refusal, `json.rs`'s heap
  stack, `xml.rs`'s namespace URI, `regex.rs`'s budget directive and `cldr.rs`'s prepared pattern —
  `docs/agent/loop-goal.toml:11621` and `:11633` are the two checks that name them.
- `Core\Random\Seeded` has no chapter of its own under `docs/reference/core/`; its members render
  from their cards and the type is introduced in `Random.md`, which is the shape `Core\Cache\Store`
  already takes. A `Random-Seeded.md` would be the wider fix if the generated page reads thin.
