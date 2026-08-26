# Handoff

## State

**Item 20's second of three changes is done.** `mwl_ir::ir::InstKind::Concat`
(`crates/mwl-ir/src/ir.rs:614`) carries a `pieces` vector rather than an `lhs`/`rhs` pair:
`Lowering::lower_concat` (`crates/mwl-ir/src/lower/expr.rs:2137`) flattens the whole `.` spine
through `Self::flatten_concat` and `lower_interpolated_parts` (`:1774`) hands its pieces over
whole, so `"<tr><td>" . $i . "</td>"` is one instruction and one allocation where it was a fold of
growing prefixes. `mwl_str_concat_n` (`crates/mwl-runtime/src/string.rs:541`) sums the total length
once and copies each piece once; `emit_concat` (`crates/mwl-codegen/src/emit.rs:1783`) keeps the
two-piece case on `mwl_str_concat` and sends three or more through a new
`Self::pointer_array_slot`. Ownership is unchanged — no piece is retained, and a fresh one is
released once the instruction has read it — which is why nothing about the temporaries stack moved.

**Measured, paired against the commit before it, 21 reps on a quiet machine**: `03-string-concat`'s
own work went 3.5 ms → 2.8 ms and its ratio 1.23× → 1.39×. `docs/perf/userland-gap.md` § B records
it and strikes the second of its three bullets; the guard is
`string::tests::an_n_ary_concatenation_allocates_one_buffer`, which asserts the exact byte count.

Verify is green (**1579** tests, 74 suites, clippy and fmt clean). `tools/leak-check.sh` was run
over a fixture exercising every n-ary shape — five-piece rows, a parenthesized nesting, a repeated
local, a three-piece interpolation, and a `toString()` that throws part way down the piece list —
and is clean.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — `[context]` in `loop-goal.toml`
has no field selecting a perf doc, and the remaining item-20 slice cites § B of it.

## Next group

Slice 1 is `crates/mwl-codegen/src/emit.rs` + `crates/mwl-runtime/src/string.rs`; slices 2 and 3
share `crates/mwl-stdlib/src/arr.rs`. [loop-goal.md](loop-goal.md) items 20 and 21 and
`docs/perf/userland-gap.md` §§ B and D are the specification.

- [ ] **A string literal stops allocating** — item 20's last third, § B's last bullet.
      `emit_const_str` (`crates/mwl-codegen/src/emit.rs:924`) calls `mwl_str_new` on every
      *evaluation*, so `$a["beta"]` in a loop allocates `"beta"` a million times. A whole
      `StrHeader` written into the data section with a pinned refcount makes it an address and no
      call at all. **The non-obvious half is the refcount reasoning, not the emission**: an
      immortal literal lives in the compiled unit, which *is* shared between requests, so
      `string.rs`'s module doc § *Why the refcount is a plain `Cell`* ("no `MwlStr` is ever
      reachable from two threads") stops being true as stated and has to say why it stays sound —
      a pinned count is never written. `mwl_str_release` (`crates/mwl-runtime/src/string.rs:728`)
      is where the pin has to be honoured, and it is on the hottest path in the runtime, so
      whatever recognises an immortal string must cost one predictable compare. Owed a valgrind
      run and an `allocated_bytes` guard.
- [ ] **`Core\Arr::map`/`filter`/`reduce` synthesize no key for a one-parameter callback** —
      item 21, § D. Each calls `mwl_array_key_at` per element
      (`crates/mwl-stdlib/src/arr.rs:886`, `:983`, `:2772`; the entry point is
      `crates/mwl-runtime/src/array.rs:1353`), which on a packed list renders a decimal and
      allocates an `MwlStr` — and `call_closure` (`crates/mwl-runtime/src/closure.rs:70`) then
      slices the argument list to the closure's declared arity and drops it. The arity is a field
      on the closure object and is readable once before the loop.
- [ ] **`Core\Arr::sort` builds no key when `preserveKeys` is false** — the same item's second
      half, same file. `preserve_keys` is `crates/mwl-stdlib/src/arr.rs:1200`.

## Backlog

- `docs/perf/history.ndjson` — item 19 still owes the append-only entry item 15 asked for
  (`docs/perf/userland-gap.md` § *This file has a lifetime*).
- Item 22: a `Core\Str` member writes its result once — `produced(&str)` allocates twice and
  `text()` re-validates UTF-8, 56 call sites each (`docs/perf/userland-gap.md` § E).
- `§12 Out::capture` is the one remaining key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.
- `orient.py`'s `[context]` manifest has no selector for `docs/perf/`, so every Stage 0 slice
  fetches the ledger by hand (`docs/agent/loop-goal.toml`).
- Conformance is 436 of the 600 Stage 4 requires; differential is 90 of 150.
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap).
