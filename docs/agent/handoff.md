# Handoff

## State

**Item 20's first of three changes is done.** `StrHeader` carries a capacity
(`crates/mwl-runtime/src/string.rs:71`), `mwl_str_append` (`:548`) writes into the spare room when
the target is solely owned and doubles when it is not, and `mwl_ir::ir::InstKind::StrAppend`
(`crates/mwl-ir/src/ir.rs:646`) carries `$s .= e` on a plain `string` local to it with **no retain
and no release** — `mwl_array_set`'s consume-one-yield-one protocol, whose one home stays
`InstKind::ArraySet`'s doc comment. `Lowering::lower_string_append`
(`crates/mwl-ir/src/lower/stmt.rs:395`) is the only lowering that takes it; every other compound
assignment, and `.=` on a property or an element, keeps the `$x = $x op e` rewrite because those
already need its write-back.

**Measured: `03-string-concat` is 1.19× where it was 0.03×**, and the two runs § B quoted went from
238 ms / 1,386 ms to 15.3 ms / 20.6 ms. `docs/perf/userland-gap.md` § B records it and strikes the
first of its three bullets. What the third header word spends — 8 bytes per string, and up to twice
the payload for one that has been appended to — is in `string.rs`'s module doc § *Capacity, and what
it spends*, not in the ledger.

Verify is green (**1576** tests, 74 suites, clippy and fmt clean). `tools/leak-check.sh` was run over
a fixture exercising all four append shapes (loop, shared binding, self-append, scalar suffix) and is
clean — the grow path's release of the consumed reference is a genuinely new refcount edge.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — `[context]` in `loop-goal.toml` has
no field selecting a perf doc, and both remaining item-20 slices cite § B of it.

## Next group — the two thirds of item 20 still open

One file set: `crates/mwl-ir/src/ir.rs`, `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-codegen/src/emit.rs`, `crates/mwl-runtime/src/string.rs`.
[loop-goal.md](loop-goal.md) item 20 and `docs/perf/userland-gap.md` § B are the specification.

- [ ] **`InstKind::Concat` becomes n-ary**, so `"<tr><td>" . $i . "</td>"` is one allocation rather
      than a fold of growing prefixes. `MwlStr::from_pieces` (`crates/mwl-runtime/src/string.rs:156`)
      already exists for it and no codegen path calls it — it needs an `extern "C"` entry point taking
      a pointer array and a count, and a stack slot to build that array in (`Self::value_slot`,
      `crates/mwl-codegen/src/emit.rs:1794`, is the nearest shape). The two producers are
      `Lowering::lower_concat` (`crates/mwl-ir/src/lower/expr.rs:2137`) and
      `lower_interpolated_parts` (`:1774`), both already left-to-right folds over operands
      `concat_operand` (`:391`) has normalized; `emit_concat` is
      `crates/mwl-codegen/src/emit.rs:1775`. Eight lowering snapshots print `concat` and will churn.
- [ ] **A string literal stops allocating.** `emit_const_str`
      (`crates/mwl-codegen/src/emit.rs:916`) calls `mwl_str_new` on every *evaluation*, so a literal
      key inside a loop allocates millions of times; its own doc comment already asks for this and
      defers it here on layout grounds. A whole `StrHeader` written into the compiled unit's data
      section with a **pinned** refcount makes it an address and no call at all. Two things this owes:
      a sentinel the retain/release/append paths recognise so a pinned count is never written (
      `mwl_str_append`'s `refcount == 1` test already declines to write a pinned literal in place, but
      `MwlStr::drop` would still decrement one), and the module-doc note item 20 names explicitly —
      an immortal literal lives in the compiled unit, the one thing a request *does* share with
      another, so `string.rs`'s § *Why the refcount is a plain `Cell`* stops being true as written
      and has to say why it stays sound.

## Backlog

- `docs/perf/history.ndjson`, the append-only per-item entry item 15 asked for — still does not exist
  (`docs/perf/userland-gap.md`).
- The suite table's median is stale by one row until the next full `tools/bench.py` run
  (`docs/perf/userland-gap.md` § *Where the suite stands*).
- Stage 0 items 21 and 22, after 20 closes (`docs/agent/loop-goal.md`).
- `§12 Out::capture` is the last outstanding spec §§ 1-12 member
  (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- Conformance is 436 of the 600 Stage 4 requires; differential is 90 of 150
  (`docs/implementation-plan.md`).
