# Handoff

## State

**Stage 0 item 21 is done end to end, and with it § D of `docs/perf/userland-gap.md`.** All four
`Core\Arr` members that call back into MWL code — `map` (`arr.rs:979`), `filter` (`:887`),
`reduce` (`:2606`) and `sort` (`:2765`) — read `mwl_runtime::closure_arity`
(`crates/mwl-runtime/src/closure.rs:124`) **once before their loop** and build a key only where the
callback declared a parameter to receive it. The preserved-key half needed a second change, since
`map` and `filter` were going to build one anyway: `mwl_runtime::SlotKey` (`array.rs:648`) answers
the key in whichever form the subject's shape already holds it — the position itself while packed —
and `store_at` (`arr.rs:1293`) writes it back through `MwlArray::set_index`, which renders nothing.
`sort` collects no keys at all when `preserveKeys` is false and no `by` closure asked for one.

**Measured, against the ledger's previous sweep (the commit before this one)**: `12-array-map-filter`
**0.36× → 0.51×**, `10-array-sort` **0.69× → 0.96×**, `11-array-sort-by-field` **3.98× → 5.12×**,
suite median **0.66× → 0.69×**. The ledger's suite table is a fresh full 9-rep sweep of the current
build. Verify is green (**1582** tests, 74 suites, clippy and fmt clean), and
`tools/leak-check.sh` is clean over a fixture exercising every callback arity of all four members
across both array shapes. The rule's home is `crates/mwl-stdlib/src/arr.rs`'s module doc
§ *A callback that does not want a key is never handed one*.

**Stage 0 has one item left: 22 / § E.** `orient.py` still does not print
`docs/perf/userland-gap.md` — `[context]` in `loop-goal.toml` has no field selecting a perf doc,
and item 22 cites § E of it.

## Next group

All three are `crates/mwl-stdlib/src/str.rs`; the third also touches `bytes.rs:425`, `path.rs:434`
and `regex.rs:678`, which are the same `produced` twice more.
[loop-goal.md](loop-goal.md) item 22 and `docs/perf/userland-gap.md` § E are the specification.

- [ ] **`text()` stops re-validating UTF-8.** § E's second bullet. `text` (`str.rs:568`) runs an
      O(n) `std::str::from_utf8` per `string` argument — 56 call sites in this file alone — over a
      buffer ADR 0009 already guarantees is well formed; its own error message says so. The
      unchecked read belongs behind one `unsafe` in `mwl_runtime::MwlStr` with the invariant named,
      not at 56 call sites.
- [ ] **`Core\Str::length` makes one pass, not three.** § E's second bullet, second half.
      `mwl_core_str_length` (`str.rs:644`) runs `is_ascii`, then a scan for `\r`, then the grapheme
      walk; only the last is unavoidable, and `strlen` is O(1) from the header.
- [ ] **`produced` writes its result once where the length is known.** § E's first bullet.
      `produced` (`str.rs:629`) allocates a `String` and then copies it into a fresh `MwlStr`.
      `join` (`:742`), `replace` (`:1105`) and `padStart` (`:1889`) each know their output length
      before they start, so each can write straight into one `MwlStr`.

## Backlog

- Item 19 still owes the append-only `docs/perf/history.ndjson` entry — `docs/perf/userland-gap.md`
  § *This file has a lifetime*.
- `§12 Out::capture` is the last key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt` —
  M4S's sink work (ADR 0092).
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- Stage 4 counts: conformance 436 of 600, differential 90 of 150 — the plan's `Open now`.
- § F (a string carries its hash) and § G (a virtual call is a slot) are the next two ledger items
  after § E, and neither is Stage 0 work.
