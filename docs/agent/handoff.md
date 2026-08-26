# Handoff

## State

**Conformance is the only frontier left, at 512 of 600.** The differential gate is met at **159** of
the 150 it requires and `python tools/gaps.py --differential` is **empty** — every spec member with a
PHP twin now has an oracle case. Verify is green (1597 cargo tests, 74 suites, clippy and fmt clean)
and runs both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run
(playbook, *Running things*).

**`Core\Arr::flattenDeep` was the last member on the differential list and it agrees with its twin
outright** — `iterator_to_array(new RecursiveIteratorIterator(...), false)`, keys and all, because
`false` discards every key alike and that is ADR 0069 § 3's rule. Not the divergence the previous
handoff predicted; the playbook bullet says why.

**The three JSON number rows that do not agree now have their case.** The refusal band, its two
bounds, the absent negative half, `1e999`, `-0`, and the writer's `.0` — all in the new
`--ORACLE-DIVERGES--` case, with the same summary as a *Divergences* playbook bullet.

**The group's third slice turned out to be already done.** `Core\Json::isValid` and
`Core\Json::decode` agreeing on every spelling is the second clause of
`tests/conformance/core/json-a-decoded-document-re-encodes-byte-for-byte.mwlt`'s `--TEST--` line, so
it was dropped rather than written twice. Read the `--TEST--` lines of a section's existing cases
before scheduling one — `sed -n 2p` over the glob is one call.

**`Core\Json` and `Core\Arr` are both deep now** (7 and 55 conformance cases), so the next group
moves to `Core\Heap`, which has 3 cases for 6 members. **`orient.py`'s `[context] modules` manifest
has no `heap.rs` selector** — add one, beside `arr.rs` and `json.rs`.

## Next group

Three slices, all `crates/mwl-stdlib/src/heap.rs` plus new files under `tests/conformance/core/`;
the third also reads `crates/mwl-stdlib/src/arr.rs`. Spec § 9 and ADR 0013 own the rules. The three
existing `heap-*.mwlt` cases pin pop order, the empty-read refusal and `foreach`; none of the below
is in them.

- [ ] **`count`, `isEmpty` and `peek` agree at every step of a push/pop sweep** — the *agreement*
      shape, counted rather than printed, so a reader that grew its own idea of the size fails here
      while still looking right on its own line. `heap.rs:617` (`count`), `heap.rs:629` (`isEmpty`),
      `heap.rs:575` (`peek`), `heap.rs:551` (`push`).
- [ ] **An interleaved push/pop sequence still pops in non-decreasing order** — the *invariance*
      shape: pushing a smaller key after a larger one has already been popped must not produce a
      pop that goes backwards, asserted by counting the non-decreasing steps over a table of
      sequences. `heap.rs:589` (`pop`), `heap.rs:516` (the constructor's comparator argument).
- [ ] **`Core\Heap`'s pop order and `Core\Arr::sort` answer the same permutation** — the
      *agreement* shape across the two members that share ADR 0013's ordering, over a table
      including ties and one comparator. `heap.rs:589`, `arr.rs:2790` (`mwl_core_arr_sort`).

## Backlog

- `Core\Csv` (2 members, 3 cases), `Core\Uuid` (5, 3), `Core\Validate` (3), `Core\Out` (1, 3) are the
  next-thinnest sections after `Core\Heap` — `docs/spec/01-core-library.md` §§ 7, 11, 12.
- `Core\Json::decodeAs<T>`'s field roster is narrower than ADR 0071 § 2's — `mwl_stdlib::json` gap 2.
- The `i64::MAX`..=`u64::MAX` refusal band's upper edge is a gap, not a rule — `mwl_stdlib::json`
  gap 1; closing it now shows up as a diff in the new divergence case.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
