# Handoff

## State

**Spec § 11 is whole.** `Core\Random::bytes` landed as `token`'s draw unrendered, and `Core\Hash::stream`
landed with `Core\Hash\Stream` — the first *mutable* `Core` instance outside § 9's collections. Its state is
the chunks `update` retained, hashed in one pass at `finish`, because a `Core` instance's slots hold only
values MWL already holds; `crates/mwl-stdlib/src/hash.rs`'s module doc owns that decision, what it spends,
and why `finish` closes the stream anyway. The spec's § 11 table gained the two instance rows it never
wrote (`$stream->update`, `$stream->finish`) and the paragraph that says a stream is consumed by `finish`.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is down to **2 keys**: `§2 from` and
`§12 Out::capture`. Conformance is 416 of 600 and differential 89 of 150. `Out::capture` still lands with
M4S's sink work, so the only registerable key left is `§2 from` — and it waits on an `Iterable`/`Iterator`
argument, which is the same thing § 9's `Core\Heap` waits on. That is the next group.

Two open temporary-lifetime gaps of one family are named where they live: `mwl_ir::lower::Lowering`'s
`owned_temporaries` field doc holds the transferred-argument case, and `landing_block`'s *Known gap* holds
the producers that still release inline. `mwl-ir`'s crate doc gap 2 is the index of both.

## Next group — the `Iterable`/`Iterator` argument, and the two members waiting on it

**Shared file set:** `crates/mwl-stdlib/src/registry.rs`, `crates/mwl-types/src/core_lib.rs`,
`crates/mwl-stdlib/src/arr.rs`, and a new `crates/mwl-stdlib/src/heap.rs`. [1] is the shape both others
declare, so it is first and the other two are then independent of each other. `crates/mwl-stdlib/src/objset.rs`
is the model for [3]: a `new`-able collection with slots, instance members and a `NEW_SYMBOL`.

- [ ] **1. An `Iterable<T>`/`Iterator<T>` parameter shape** — [ADR 0053](../adr/0053-iteration-and-generators.md)
      § 2 owns what the two interfaces are; `docs/spec/01-core-library.md:256` is the row that spells the
      union. A new `CoreTy` variant beside `Instance` at `registry.rs:89`ff, mapped in
      `crates/mwl-types/src/core_lib.rs:282`'s match beside `CoreTy::Instance`. The question to settle and
      record in `registry::CoreTy`'s own doc: whether a plain `array<T>` satisfies it (it must, or
      `Arr::from($array)` stops compiling) and how the helper *reads* one at the ABI — an `MwlArray`, or a
      generator object it has to drive.
- [ ] **2. `Core\Arr::from`** — spec row at `docs/spec/01-core-library.md:256`,
      `from(Iterable<T>|Iterator<T> $items, {limit?: uint}): array<T>`. Registry row in `arr.rs`'s `CLASS`
      (its block ends at `arr.rs:506`), and this strikes the ratchet's `§2 from`. The options bag flattens
      to one argument per option, so the row is `params` 2 and the helper is `args: [2]`.
- [ ] **3. `Core\Heap`** — `docs/spec/01-core-library.md:685` (`push`, `peek`, `pop`, `count`, `isEmpty`)
      and the ordering rule at `:692`, which is [ADR 0013](../adr/0013-comparable-interface.md)'s
      `Comparable`. `registry.rs:822`'s `GENERIC_CLASSES` already declares it as `["T"]`; it needs a row in
      `CLASSES` (`registry.rs:669`) and in `CONSTRUCTORS` (`registry.rs:708`). Its three `Iterable` rows are
      what [1] unblocks.

## Backlog

- Stage 4's counts are their own work — conformance 416 of 600, differential 89 of 150 (plan, *Open now*).
- § 6 owes `Core\Json::decodeAs<T>`; `crates/mwl-stdlib/src/json.rs` gap 2 says what waited and no longer does.
- § 10 owes the constructor's `{previous: $e}` options shape and `$e->location` (ADR 0071 § 5).
- § 12's `Core\Out::capture` lands with M4S's sink work (ADR 0092), not before it.
- `mwl-stdlib` rows carry no ADR 0088 qualifier classification, so `Core\Str::format` is not yet a sink.
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1's neighbour).
