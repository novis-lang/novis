# Handoff

## State

**Conformance is at 516 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**§ 12's `Core\Csv` is done — 5 cases for 2 members.** The two new ones are the *invariance* and
*edges* shapes: `format` then `parse` is the identity over a table of awkward records (12 fields
compared by counting, including a field holding the separator, the quote, a bare LF and a bare
CRLF, plus an empty one and a space-padded one), the document is a fixed point of re-formatting,
and the one record the identity fails for is named — a record of a *single* empty field writes a
blank line and a blank line is not a record. The second pins that a trailing newline never changes
what a document holds, under both `header` readings, by comparing whole re-formatted structures
rather than record counts.

**The § 9 leftover was not taken again** — it is item 1 below, unchanged and anchored. Context was
never the reason; the two `Core\Csv` slices simply filled the group.

## Next group

Three slices. **Item 1 is the carried § 9 leftover** and reads `heap.rs` + `arr.rs`, so take it
alone or first. **Items 2 and 3 share `crates/mwl-stdlib/src/validate.rs`** — 6 members over 3
cases, the thinnest section left now that `Core\Csv` is 5 — plus new files under
`tests/conformance/core/`; spec § 12's first table owns the rules and the crate's module doc owns
every `filter_var` divergence. `validate-members.mwlt` is one broad 99-line case walking all six
members' lines, so a new one takes a depth shape rather than another row of that.

- [ ] **`Core\Heap`'s pop order and `Core\Arr::sort` answer the same permutation** — the
      *agreement* shape across two members implementing one rule (ADR 0013): drain a heap into an
      array, sort the same table with `Core\Arr::sort`, and count the positions that agree rather
      than printing either sequence. `heap.rs:589` (`pop`), `arr.rs:2790` (`sort`).
- [ ] **`Core\Validate`'s six predicates agree about the subject no line accepts** — the
      *agreement* shape, counted: the empty string, a string that is only whitespace, and one
      holding a NUL are each asked of all six, asserting that they **agree** rather than what each
      answered. `validate.rs:388` (`isEmail`), `:402` (`isDomain`), `:420` (`isIp`), `:436`
      (`isMac`), `:450` (`isAscii`), `:465` (`isPrintable`).
- [ ] **`isAscii` and `isPrintable` are bounds asserted on both sides** — the last code point each
      accepts and the first it refuses, named together in one case: `~` (7E) against DEL (7F) for
      `isPrintable`, DEL (7F) against U+0080 for `isAscii`, and where the two disagree in between.
      `validate.rs:450`, `validate.rs:465`.

## Backlog

- `Core\Uuid` and `Core\Out` are 3 cases each — the thinnest sections after `Core\Validate`
  (`python tools/gaps.py` is the worklist; do not re-derive it).
- `Core\Json::decodeAs<T>`'s decoder reads scalar-fielded classes only — plan, *Open now*; ADR 0071.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `do`/`while` does not lower, and a closure cannot be called through the variable holding it —
  `mwl-ir` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
