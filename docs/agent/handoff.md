# Handoff

## State

**Conformance is at 542 of 600, and it is the only frontier left.** Verify is green (1597 cargo tests,
74 suites, 542 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself,
so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

This session added no library code. It landed the first two of the three `Core\Path` slices the previous
handoff named: `path-relative-to-and-join-undo-each-other.mwlt` (20 pairs whose round trip through
`join` and `normalize` returns the path, 12 pairs asserting a relative path exists **iff** the two
arguments agree about `isAbsolute`, and the other refusal — a base still holding a `..` — named against
the absolute spelling where `normalize` drops it) and `path-dirname-and-basename-partition-a-path.mwlt`
(20 paths recomposed through the normal form, so a bare name and a bare root stop being exceptions;
names asserted separator-free under both spellings; `{levels:}` checked against walking up one level at
a time at every depth, and at both ends of what it accepts). The third, the `withExtension`/`extension`
inverse pair, was not taken and heads the next group.

**`Core\Path` is spec § 8, not § 7** — the previous handoff and one case's `--TEST--` line said § 7;
the case is fixed, and `docs/spec/01-core-library.md` is the home.

`orient.py`'s pack was complete for this work; nothing was fetched outside it beyond `path.rs` and two
sibling `Core\Path` cases.

## Next group

Three slices, and the file set changes once inside it: **item 1 stands alone on
`crates/mwl-stdlib/src/path.rs`** and closes `Core\Path`, **items 2 and 3 share
`crates/mwl-stdlib/src/time.rs`** — so a session taking two should take 2 and 3 together, not 1 and 2.
Each writes a new file under `tests/conformance/core/`. `docs/spec/01-core-library.md` § 8 owns the
`Path` rules and § 4 the `Time` ones. Items 2 and 3 are where the frontier actually is:
`gaps.py --coverage` puts `Core\Time\Instant` at 0.00 cases per member with three members no case calls
at all, and `Core\Time\DateTime` at 0.06 with three more.

- [ ] **`withExtension` and `extension` are inverses wherever the name has one** —
      `extension(withExtension($p, $e)) == $e` over a table, `withExtension($p, null)` removes it, and
      a dotfile and a trailing dot are where the pair stops agreeing (`stem_and_extension` owns both
      divergences in one place, so a case asserting them separately is asserting one rule).
      `crates/mwl-stdlib/src/path.rs:90` (`extension`), `:97` (`withExtension`),
      `:296` (`stem_and_extension`).
- [ ] **`Core\Time\Instant` orders and subtracts consistently, and `in` is the only zone question** —
      `compareTo` agrees with the ordering `minus` implies over a table of instants, and `in` moves the
      rendering without moving the instant. `crates/mwl-stdlib/src/time.rs:1793` (`minus`),
      `:1819` (`compareTo`), `:2450` (`in`).
- [ ] **`Core\Time\DateTime`'s parts agree with each other** — `date`, `dayOfYear` and `difference`
      asked of one table and asserted against each other rather than row by row (a `dayOfYear` that is
      the count of days since `date`'s own January 1, a `difference` that is what re-adding undoes).
      `crates/mwl-stdlib/src/time.rs:2330` (`difference`), `:2366` (`date`), `:2407` (`dayOfYear`).

## Backlog

- `Core\Uri` is 0.53 cases per member and `Core\Validate` 0.83 — `docs/spec/01-core-library.md` §§ 12, 6.
- `Core\Time\Duration` (0.37) and `Core\Time\Date` (0.33) sit in the same file as the group's items 2-3.
- 9 differential candidates remain (`gaps.py --differential`); three of the six `Core\Time` ones have no
  oracle a case could freeze.
- `Core\ObjectMap` (0.78) and `Core\ObjectSet` (0.89) — spec § 9.
- 57 of the 58 unasserted `Fault::` sites are `fatal` and so unreachable by any case (`gaps.py --errors`).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
