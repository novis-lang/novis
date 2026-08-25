# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0* holds
items 1 to 17; **1 to 14, 16 and 17 are done**, so what is open is **item 15 alone**. `loop.py`
short-circuits at stage 0, so **Stage 3 is shut until it clears**.

**Item 15's runtime half is done, and the previous handoff understated it.** All four named tests exist and
pass, `an_integer_subscript_allocates_no_key` included: `mwl_array_get_index` and `mwl_array_set_index` sit
beside the key-taking pair at `crates/mwl-runtime/src/array.rs:1129` and `:1212`, they answer from
`Shape::Packed` with no key rendered, and `helpers.rs:1207` registers both. `crates/mwl-runtime/src/array.rs`
holds the two shapes behind one `Table` — `Shape::Packed(Vec<Value>)` for keys exactly `"0"`…`"n−1"` in
order, today's ordered hash for everything else — with `Table::hashed_mut` the **only** place the invariant
is given up. That file's module doc owns the decision, the PHP comparison and what it spends; nothing about
it is restated here.

**What item 15 still owes is the codegen half and the measurement.** `crates/mwl-codegen/src/emit.rs`
names neither new symbol, so compiled code still builds a key string and the ABI pair is dead weight until
it does; and `docs/perf/history.ndjson` does not exist, only `docs/perf/userland.ndjson`.

**The extension roster is settled** — ADR 0051 § 3 went through a full placement pass with the user, one
PHP extension at a time, sorted by real-world usage. No Core or Native milestone moved. Tier 1 shrank to
**two first-party components** and everything else in that column was dropped with a named replacement or
explicitly deferred. That ADR's body is the rule; this is not the place for the list.

Verify is green (**1560** tests, 74 suites, clippy and fmt clean) — fmt only after this session fixed a
signature in `array.rs` that was committed unformatted. Conformance **435**, differential **90** —
untouched, no `.mwlt` case was added or edited.

## Next group — item 15's last two slices

The first is **two call sites plus a signature declaration**; the second is a measurement and a new file.
They are not the same file set, so do the codegen one first and stop if it fills the session.
[loop-goal.md](loop-goal.md) item 15 is the specification.

- [ ] **Call the integer subscript path from codegen.** The runtime pair exists and is unreferenced.
      Anchors: `crates/mwl-codegen/src/emit.rs:1849` (`ArrayGet`) and `:1824` (`ArraySet`), with the
      signatures declared at `crates/mwl-codegen/src/lib.rs:604` — declare `mwl_array_get_index` and
      `mwl_array_set_index` there the way the key-taking pair is declared. Emitting the new pair needs the
      subscript's static type to be an `int`/`uint` at the emit site; if that is not on hand, say so in the
      handoff rather than widening the IR inside this slice. The runtime side is
      `crates/mwl-runtime/src/array.rs:1129`, `:1212` and `:236` (`packed_index`).
- [ ] **The first `docs/perf/history.ndjson` entry**, with a `php_ratio`, per
      [ADR 0026](../adr/0026-performance-measurement-methodology.md). That file not existing is why nothing
      caught this. The four rows to re-measure are the ones in `array.rs`'s own module doc table
      (`$a[] = $v`, `$a[$i]`, `$a['name']`, `foreach`), against the PHP 8.5.9 oracle on this machine; the
      table's "unpacked" column is the before.

## Backlog

- `Core\Fatal::onLimit` and the `[limits] fatal_reserve_*` directives — ADR 0020 § 1, M4S/M7.
- Spec § 13's `isBoolean` replacement — `$s as ?bool` does not exist; ADR 0035 makes `as bool` total.
- `Core\Out::capture` is the one key left in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`,
  and `examples/collect.mwl:47` is where Stage 3 resumes once item 15 clears.
