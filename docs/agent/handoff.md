# Handoff

## State

**Stage 0 item 20 is done end to end, and with it § B of `docs/perf/userland-gap.md`.** A string
literal no longer allocates: `emit_const_str` (`crates/mwl-codegen/src/emit.rs:924`) puts a whole
`StrHeader` and the payload into the unit's data section through the new
`Emitter::emit_immortal_str` and materializes its address — no call, no allocation — and
`emit_array_new` (`:1893`) gives an array literal's keys the same treatment. The header's refcount
is `mwl_runtime::IMMORTAL_REFCOUNT` (`crates/mwl-runtime/src/string.rs:144`), which `MwlStr`'s
`Clone` and `Drop` and `mwl_str_retain` each compare against and step over; the data object is
declared **not writable**, so a lapse faults rather than corrupting a word two requests share.
`mwl-codegen` no longer emits `mwl_str_new` at all, so `RuntimeSig::StrNew` and `Signatures::str_new`
are gone.

**Why that is sound is the half worth knowing**, and it lives in `string.rs`'s module doc
§ *An immortal string, and why the `Cell` survives it*: a compiled unit **is** shared between
requests, so "no `MwlStr` is ever reachable from two threads" is now stated as the narrower claim
that actually holds — no refcount two threads can reach is ever written.

**Measured, paired against the commit before it**: `$a["beta"]` went 26.6 ns → **21.3 ns**,
`04-string-format` 100.0 → **87.2 ms** of work (21 reps), and `05`, `06`, `07`, `15`, `17` about 5%
each; `03-string-concat` did not move at this resolution. The suite table in the ledger is a **fresh
full 9-rep sweep** of the current build rather than the stale pre-allocator one, and its median is
**0.66×**.

Verify is green (**1581** tests, 74 suites, clippy and fmt clean). `tools/leak-check.sh` was run
over a fixture exercising `.= ` onto a literal, array-literal keys, a literal aliased into two
locals and an empty literal, and is clean.

**`orient.py` still does not print `docs/perf/userland-gap.md`** — `[context]` in `loop-goal.toml`
has no field selecting a perf doc, and every remaining Stage 0 item cites a section of it.

## Next group

All three share `crates/mwl-stdlib/src/arr.rs`; the first two also read
`crates/mwl-runtime/src/closure.rs:70` (`call_closure`, where the arity slice happens).
[loop-goal.md](loop-goal.md) item 21 and `docs/perf/userland-gap.md` § D are the specification.

- [ ] **`Core\Arr::map`/`filter`/`reduce` synthesize no key for a one-parameter callback.** § D's
      first paragraph. `mwl_core_arr_filter` (`arr.rs:862`), `mwl_core_arr_map` (`:959`),
      `mwl_core_arr_reduce` (`:2577`) each call `mwl_array_key_at` per element, which renders a
      decimal and allocates an `MwlStr` on a packed list, and `call_closure` then slices the
      argument list to the closure's declared arity and drops it. The arity is a field on the
      closure object and is readable once before the loop.
- [ ] **`Core\Arr::sort` builds no key when `preserveKeys` is false.** § D's second paragraph.
      `mwl_core_arr_sort` (`arr.rs:2723`), with `preserve_keys` (`:1200`) and `key_bytes` (`:1233`)
      the two helpers involved.
- [ ] **Item 22 / § E — a `Core\Str` member writes its result once.** Different file
      (`crates/mwl-stdlib/src/str.rs`), so take it only as a fresh session's first slice.

## Backlog

- Item 19 still owes the append-only `docs/perf/history.ndjson` entry item 15 asked for —
  `docs/agent/loop-goal.md`.
- The ledger's own rule deletes a landed row; §§ A, B and C are all landed and are still there,
  struck rather than removed — `docs/perf/userland-gap.md` § *This file has a lifetime*.
- The plan's `Status` field says 435 conformance cases and the tree has 436 —
  `python tools/session.py --check`.
- § 12 owes `Core\Out::capture`, the last key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `docs/implementation-plan.md`.
- `do`/`while` does not lower — `mwl-ir`'s own module doc.
