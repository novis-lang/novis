# Handoff

## State

**Conformance is at 552 of 600, and it is the only frontier left.** Verify is green (1597 cargo
tests, 74 suites, 552 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt`
trees itself, so after a green `verify.py` there is nothing else to run (playbook, *Running
things*).

This session took **two** slices, both over `mwl_stdlib::ordering`'s one helper and its call sites
in `arr.rs`, `math.rs` and `heap.rs`, and added no library code. It ended near 55k of the 200k
ceiling. Both landed cases are conventions.md's *agreement* shape — one question asked of every
member that shares a rule, counted rather than echoed — and both findings are a bullet under
playbook § *Divergences and refusals already pinned*, which is where the detail lives:
`ordering-is-one-total-order-shared-by-arr-math-and-heap.mwlt` (432 pairwise agreements per table,
plus the whole-table drain) and
`ordering-refuses-a-pair-with-no-order-once-for-all-seven-members.mwlt` (56 agreements over eight
rows, six refusing and two accepting).

**The by-hand `docs/adr/` pass has landed** — the 103 modified ADRs the last five handoffs warned
about are committed and the tree is clean. There is nothing to avoid staging any more.

**`gaps.py --coverage`'s thinnest-class ranking is still not a worklist** — every §§ 1–12 class has
had a pass, so the ratio measures member count rather than depth, and the seam with room left is
conventions.md's *agreement* shape. The two cases above are the first pair written to it
deliberately; the group below is the third.

`orient.py`'s pack was complete. What was fetched outside it: `ordering.rs`'s `compare_values` and
`comparator_sign`, the `extremum`/`pick`/`compare` call sites, and the `Core\Heap` registry rows.

## Next group

Three slices on **one file set** — `crates/mwl-stdlib/src/arr.rs`, `crates/mwl-stdlib/src/ordering.rs`
and `crates/mwl-stdlib/src/heap.rs`, the same three this session had open. [2] and [3] share every
anchor. Nothing here needs a new member.

- [ ] **`Core\Arr::sort`'s one deliberate divergence from PHP's `sort`** — the doc comment above
      `crates/mwl-stdlib/src/arr.rs:2790` names it and `:2894` is the natural-order call it rests
      on; strings compare bytewise, never numerically, so PHP's `sort(["10", "9"])` and MWL's
      disagree. Its own file, because the gate counts files, and `php -r` settles the oracle side
      while authoring (the case itself stays in `tests/conformance/`, no `--ORACLE--`).
- [ ] **One comparator contract, two receivers** — `crates/mwl-stdlib/src/ordering.rs:78`
      (`comparator_sign`) is what both `Core\Arr::sort`'s `{comparator: …}` (the wrapper at
      `arr.rs:3065`, the member at `arr.rs:2790`) and `Core\Heap`'s constructor comparator
      (`heap.rs:285` `sign_of`, `heap.rs:516` `mwl_core_heap_new`) read, so an `int`, a `uint` and a
      `float` verdict of the same sign must order identically through both, and a `NaN` verdict must
      refuse from both naming its own member. Counted agreements, one table, both receivers.
- [ ] **A tie names the same entry everywhere** — `extremum`'s "the first extreme wins"
      (`arr.rs:4357`) and `merge_sort`'s stability (`arr.rs:3069`) are one rule, and `sortByKey`
      (`arr.rs:2955`) is the third member bound by it. Ties between equal *values* are invisible, so
      the case sorts a keyed array and asserts on `Core\Arr::keys` of the result.

## Backlog

- Nine spec members whose **Replaces** column gives them a PHP twin no oracle case calls, all
  `Core\Time` and `Core\Encoding` — `python tools/gaps.py --differential`.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0086 § 1's substitution table is unbuilt — `crates/mwl-stdlib/src/cli.rs` gap 1.
