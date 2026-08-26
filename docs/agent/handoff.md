# Handoff

## State

**Stage 0 item 19 is done bar its ledger entry.** An `int` subscript now reaches
`mwl_array_get_index`/`mwl_array_set_index` with nothing rendered and nothing allocated:
`Lowering::lower_array_key` (`crates/mwl-ir/src/lower/expr.rs:1876`) hands the `int` straight to
`InstKind::ArrayGet`/`ArraySet`, and `mwl-codegen` picks the primitive off the key operand's own
`Ty` (`crates/mwl-codegen/src/emit.rs:687` and `:1848`). There is no second instruction and no
key-kind field — `mwl-ir`'s module doc § *an array key is a `string`, and an `int` subscript no
longer spells it* owns that shape decision and the two subscripts that **still** render: a `uint`,
because the index ABI is an `i64` and a `uint` above `i64::MAX` has no `i64` spelling naming the
same key, and any key reaching `InstKind::ArrayUnset`, which has no index-shaped primitive beside
it. Both are pinned by their own IR snapshot test.

**Measured, not projected: 6.3 ns against the rendered path's 28.5 ns**, an A/B of `$a[$i]` and
`$a[$i as uint]` inside one release binary over the same packed array. That is 22.2 ns off every
integer subscript, and `docs/perf/userland-gap.md` § A now records it. The suite rows do not move
for it — `08`/`09`/`12`/`13` measure 0.69×/0.40×/0.34×/0.47×, where the pooled-allocator column
already put them — because those cases reach elements through `foreach` and a `string` key.

Verify is green (**1568** tests, 74 suites, clippy and fmt clean); the two new tests are the two
still-renders snapshots. **No valgrind run, deliberately**: this change removes an allocation and
the release that matched it and adds no refcount edge at all, so there is no new edge to check.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — `[context]` in `loop-goal.toml`
has no field selecting a perf doc, and items 20-22 each cite a section of it.

## Next group — item 20, a string has capacity and `.=` appends into it

One file set: `crates/mwl-runtime/src/string.rs`, `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-codegen/src/emit.rs`. [loop-goal.md](loop-goal.md) item 20 is the specification, and it
names three changes over one layout revision. `03-string-concat` is 0.03× and is the one case no
allocator fixes: 50,000 appends take 238 ms and 100,000 take 1,386 ms.

- [ ] **`StrHeader` carries capacity, and `mwl_str_append` appends into it.**
      `crates/mwl-runtime/src/string.rs:54` is the header, `:413` is `mwl_str_concat`. The append
      takes `mwl_array_set`'s own *consume one reference, return one* protocol, so the holder is
      re-pointed at the result with no retain or release — `mwl_ir::ir::InstKind::ArraySet`'s doc
      comment is the worked statement of it.
- [ ] **`InstKind::Concat` becomes n-ary**, so `$a . $b . $c` is one allocation.
      `crates/mwl-ir/src/lower/expr.rs:2137` (`lower_concat`) and `:391` (`concat_operand`) on the
      IR side, `crates/mwl-codegen/src/emit.rs:1771` (`emit_concat`) on the other.
      `MwlStr::from_pieces` already exists for exactly this and codegen never calls it.
- [ ] **A string literal stops allocating.** `emit_const_str`'s own doc comment already asks for it
      and defers to `mwl-runtime` on layout grounds. **Write down** what item 20's own paragraph
      says must not be re-derived: an immortal literal lives in the compiled unit, which is the one
      thing a request *does* share with another, so `string.rs`'s "no `MwlStr` is ever reachable
      from two threads" reasoning behind the plain `Cell` refcount stops being true as stated. It
      stays sound only because a pinned refcount is never written, and the module doc has to say so.

## Backlog

- The first `docs/perf/history.ndjson` entry, with a `php_ratio` — item 19's last slice and item
  15's, `python tools/bench.py --json <path>` writes the record. [loop-goal.md](loop-goal.md):196.
- Item 21, no key synthesized for a callback that does not want one — `Core\Arr::map`/`filter`/
  `reduce`/`sort`. [loop-goal.md](loop-goal.md):216.
- Item 22, a `Core\Str` member writes its result once — `produced(&str)` and `text()`, 56 call
  sites each. [loop-goal.md](loop-goal.md):222.
- `mwl_array_unset_index` does not exist, so `unset($a[$i])` still renders a decimal. A deliberate
  non-widening, not an oversight — `mwl-ir`'s module doc says why.
- A positional element in an array literal that also has an explicit key still emits a `ConstStr`
  decimal per element (`crates/mwl-ir/src/lower/expr.rs:3275`); the same `Ty::Int` operand would do.
- `§12 Out::capture` is the whole remaining machine-readable work list for spec §§ 1-12 —
  `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.
