# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 14, 16 and 17 are done**, so what is open is **item 15 alone**.
`loop.py` short-circuits at stage 0, so **Stage 3 is shut until it clears**.

**Item 15 is packed, degrading and pinned as equivalent — three of its four tests are green.**
`crates/mwl-runtime/src/array.rs` now holds two shapes behind one `Table`: `Shape::Packed(Vec<Value>)`
for an array whose keys are exactly `"0"`…`"n−1"` in order, and today's ordered hash for everything
else. `Table::hashed_mut` is the **only** place the invariant is given up, so a gap, a non-numeric
key, a `"08"`, a negative key or a removal from the middle is one call to it and then the existing
path unchanged. That file's module doc owns the decision, the PHP comparison and what it spends;
nothing about it is restated here.

**PHP's append counter moved onto the shared `Table`**, out of the hash, because it is the one thing
a packed array cannot re-derive from its own length: `$a = [1,2,3]; unset($a[2]); $a[] = 9;` writes
key `3` in PHP 8.5, so a tail removal keeps the packed form and the *append after it* is what
degrades. That is pinned by `a_non_sequential_key_degrades_the_packed_array`.

**What is still owed on item 15 is the ABI half**, below: `an_integer_subscript_allocates_no_key`
is the one named test that does not exist, because compiled code still has to build a key string
before it can call `mwl_array_get`/`mwl_array_set` at all. Nothing depends on the current ABI set
yet, which is exactly the window the module doc says the addition has to land in.

Verify is green (**1558** tests, 74 suites, clippy and fmt clean). Conformance **435**, differential
**90** — untouched, no `.mwlt` case was added or edited. `mwl test tests/` reports 519 passed /
6 failed, the same six PHP-on-Windows oracle failures as before, and `tools/leak-check.sh` under
valgrind is clean on `examples/arrays.mwl`, `iterate.mwl` and `report.mwl` — the new refcount edges
are `Table::separate`'s packed arm and `Table::take_values`.

## Next group — item 15's last two slices

The first is **one file plus its two callers**; the second is a measurement and a new file. They are
not the same file set, so do the ABI one first and stop if it fills the session.
[loop-goal.md](loop-goal.md) item 15 is the specification.

- [ ] **The integer subscript path** — `mwl_array_get_index(array, i64, out)` and
      `mwl_array_set_index(array, i64, value) -> *mut ArrayHeader` beside today's key-taking pair,
      answering from `Shape::Packed` with no key rendered and degrading to a synthesized key only
      when the shape is `Hashed`. Test `an_integer_subscript_allocates_no_key`, `-p mwl-runtime`,
      written the way `a_list_shaped_array_holds_no_index_map` is — a `live_bytes()` delta, not a
      claim. Anchors: `crates/mwl-runtime/src/array.rs:1010` (`mwl_array_get`), `:1061`
      (`mwl_array_set`), `:1095` (`mwl_array_append`), `:236` (`packed_index`, which is the
      key→position half already written). The two call sites that make it pay:
      `crates/mwl-codegen/src/emit.rs:1849` (`ArrayGet`) and `:1824` (`ArraySet`), with the
      signatures declared at `crates/mwl-codegen/src/lib.rs:604`. Emitting the new pair needs the
      subscript's static type to be an `int`/`uint`, so if that is not on hand at the emit site,
      land the primitives and their test and say so — the ABI is the part that expires.
- [ ] **The first `docs/perf/history.ndjson` entry**, with a `php_ratio`, per
      [ADR 0026](../adr/0026-performance-measurement-methodology.md). That file not existing is why
      nothing caught this. The four rows to re-measure are the ones in `array.rs`'s own module doc
      table (`$a[] = $v`, `$a[$i]`, `$a['name']`, `foreach`), against the PHP 8.5.9 oracle on this
      machine; the table's "unpacked" column is the before.

## Backlog

- `Core\Fatal::onLimit` and the `[limits] fatal_reserve_*` directives — ADR 0020 § 1, M4S/M7.
- Spec § 13's `isBoolean` replacement — `$s as ?bool` does not exist; ADR 0035 makes `as bool` total.
- `Core\Out::capture` is the one key left in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`,
  and `examples/collect.mwl:47` is where Stage 3 resumes once item 15 clears.
