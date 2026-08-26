# Handoff

## State

**Conformance is at 518 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**The § 9 leftover is landed and § 12's `Core\Validate` is at 5 cases.** The first is the
*agreement* shape across two members implementing one rule: a heap drained by `pop` and the same
table given to `Core\Arr::sort` are compared position for position under four orderings — natural,
reversed (`{order: Core\Order::Desc}` against a comparator), a key written twice (`{by}` against a
comparator over the same static helper), and bytewise over strings — with every heap asserted
drained and one crossed pair scoring 0 of 6 so the counter is shown able to say no. The second asks
one question of all six `Core\Validate` predicates at once: six degenerate subjects (empty, three
whitespace, a NUL-holding one, a run of spaces) are refused unanimously by the four structural
predicates (24 of 24), are all six ASCII, and split 3 of 6 on `isPrintable` — the count that says
the two character-class predicates are not one predicate asked twice.

**Item 3 below is the carried leftover this time**, and the group's second and third slices move on
to `Core\Uuid`, which is 3 cases for 4 members plus `toString`.

## Next group

Three slices. **Item 1 is the carried § 12 leftover** and reads `crates/mwl-stdlib/src/validate.rs`
alone, so take it first. **Items 2 and 3 share `crates/mwl-stdlib/src/uuid.rs`** — 4 members plus
`toString` over 3 cases, the thinnest section left with more than one member — plus new files under
`tests/conformance/core/`; spec § 11 owns the rules (and says there is no `isValid`) and the crate's
module doc owns every divergence from PHP's `uniqid`/`random_bytes` idiom.

- [ ] **`isAscii` and `isPrintable` are bounds asserted on both sides** — the last code point each
      accepts and the first it refuses, named together in one case: `~` (7E) against DEL (7F) for
      `isPrintable`, DEL (7F) against U+0080 for `isAscii`, and where the two disagree in between.
      `validate.rs:450` (`isAscii`), `validate.rs:465` (`isPrintable`).
- [ ] **`Core\Uuid::v7` is time-ordered across a sweep, not just across two draws** — the
      *invariance* shape: draw a table of v7s in a loop, compare each against the one before by its
      leading timestamp field, and **count** the non-decreasing pairs rather than printing any of
      them. `uuid.rs:309` (`v7`), `uuid.rs:385` (`toString`).
- [ ] **`parse` and `tryParse` agree about every subject, and disagree only in how they say no** —
      the *agreement* shape over a table of canonical and malformed forms: `tryParse` answering
      `null` exactly where `parse` throws, counted. `uuid.rs:334` (`parse`), `uuid.rs:362`
      (`tryParse`).

## Backlog

- `Core\Csv` and `Core\Out` are the next-thinnest after `Core\Uuid` — 5 cases for 2 members and 3
  for 1 — `docs/spec/01-core-library.md` § 12.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is on no `mwl-stdlib` member row (`mwl_stdlib::hash` doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
