# Handoff

## State

**Conformance is at 514 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**§ 9's `Core\Heap` is deep now — 5 conformance cases for 6 members.** The two new ones are the
*agreement* and *invariance* shapes over a push/pop table: `count`, `isEmpty`, `peek` and ADR 0053
§ 3's cursor agree at every step (65 agreements over 13 steps, on both orderings), and every `pop`
answers the element that sorts first among everything still held (10 pairs, both orderings), with a
monotone table draining non-decreasing. Both are counted, not printed.

**A `.mwlt` case can factor a sweep into a `public static function` taking the collection itself.**
`Sweep::run(Core\Heap<int> $h, array<int> $ops): int` lowers, `foreach` over that parameter inside
the loop lowers, and `($h->count() as int) == $size` is how a `uint`-answering `count` is compared
against an `int` counter the case keeps. Nothing here needed a workaround, so no playbook bullet.

**The group's third slice was not taken** — it is the third item below, unchanged and anchored.
`[context] modules` in `loop-goal.toml` now names `csv.rs` and `heap.rs`; `json.rs` came out, § 6
being deep at 7 cases and both its remaining `Fault::` sites hidden by the codec case's stem.

## Next group

Three slices. **The first two share `crates/mwl-stdlib/src/csv.rs`** — 2 members, 3 cases, the
thinnest section left — plus new files under `tests/conformance/core/`; spec § 12's second table
owns the rules. The third is the § 9 leftover and reads `heap.rs` + `arr.rs` instead, so take it
alone or first. The existing `csv-*.mwlt` three pin RFC 4180 round-tripping, a ragged record's keys
and a non-default dialect; none of the below is in them.

- [ ] **`Core\Csv::format` then `Core\Csv::parse` is the identity over a table of awkward records** —
      the *invariance* shape, counted rather than printed: a field holding the separator, one
      holding the quote, one holding a newline, an empty field, one with leading and trailing
      space, and one that is nothing but a quote. `csv.rs:452` (`format`), `csv.rs:354` (`parse`).
- [ ] **`Core\Csv::parse`'s edges** — the *edges* shape: empty text, a header-only document under
      `header: true`, the same document with and without its trailing newline, and a record of one
      empty field. `csv.rs:354`.
- [ ] **`Core\Heap`'s pop order and `Core\Arr::sort` answer the same permutation** — the
      *agreement* shape across two members implementing one rule (ADR 0013): drain a heap into an
      array, sort the same table with `Core\Arr::sort`, and count the positions that agree rather
      than printing either sequence. `heap.rs:589` (`pop`), `arr.rs:2790` (`sort`).

## Backlog

- `Core\Validate`, `Core\Uuid` and `Core\Out` are 3 cases each — the thinnest sections after
  `Core\Csv` (`python tools/gaps.py` is the worklist; do not re-derive it).
- `Core\Json::decodeAs<T>`'s decoder reads scalar-fielded classes only — plan, *Open now*; ADR 0071.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `do`/`while` does not lower, and a closure cannot be called through the variable holding it —
  `mwl-ir` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
