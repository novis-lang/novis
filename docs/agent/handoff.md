# Handoff

## State

**Stage 4's two counts are the frontier — conformance 482 of 600, differential 90 of 150** — and the
gap is behavioural depth per member, not coverage: every registered member already has a case, and both
of Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **482 passed, 0 failed** and `mwl test tests/` is **572 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook, twice),
and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than it (playbook,
*Running things*).

**The depth pass has reached every section**, so what matters from here is the *shape* a new case takes,
not which section it lands in; the four shapes are named in the plan's *Open now*, and the agreement
shape — one question asked of every member that shares it — now carries six cases and is still the one
with the most room. **A pass must land its claim as a new case file** (playbook, *Writing a test case*).

Three spellings a case cannot use: a closure called through the variable holding it, the first-class
callable `Class::method(...)` (both `mwl-ir` gap 1 — declare a `class` with a `public static function`
and call it *directly*, which every `arr` depth case does), and `bool as int`. What a case *can* do, and
both of this session's use: a `?T` narrowed by an `if ($v == null) { return …; }` and then converted
with `as string`; a closure passed to a `Core` member capturing the enclosing static method's own
parameter; and `Core\Arr::count($a) as int` wherever a `uint` meets an `int` (playbook has the `E0407`).

## Next group

All three share `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`, and each reuses one of
*Open now*'s four shapes. Take them in this order.

- [ ] **`isList` and the members that answer one** — `isList` (`arr.rs:1431`), `values`
      (`arr.rs:2231`), `keys` (`arr.rs:1488`), `flip` (`arr.rs:2039`), `chunk` (`arr.rs:1704`),
      `flatten` (`arr.rs:2084`), `appendAll` (`arr.rs:3757`), ADR 0007 § 5: a list is exactly the keys
      `"0" … "n−1"` in order, which is also `Core\Json::encode`'s array test — so the members that
      renumber answer a list from any subject and the members that preserve keys answer one only when
      they were given one. The agreement shape: one question, every member, one tally.
- [ ] **`range`'s two ends and its step** — `range` (`arr.rs:2364`), spec § 2, the bound-on-both-sides
      shape: both endpoints are *in* the range, a step that does not divide the span stops before
      overshooting rather than past it, a descending range is `from > to` rather than a negative step,
      and a step of `0` is the refusal one past the first that terminates. `Core\Arr::count` agreeing
      with the span-over-step arithmetic on every row is what makes it one sweep rather than a table.
      `arr-count-and-range.mwlt` and `arr-range-counts-up-down-and-by-step.mwlt` are what exist.
- [ ] **The array constructors agree about length and keys** — `fill` (`arr.rs:2277`), `fillKeys`
      (`arr.rs:2301`), `from` (`arr.rs:2419`), `column` (`arr.rs:2182`), spec § 2: each builds a
      subject without being given one, so the shared question is what it holds — `fill` answers a list
      of a stated length, `fillKeys` answers the keys it was handed, `from` answers what the sequence
      drained, and `column` answers one cell per row *that has the key*, which is the one of the four
      whose count can differ from its input's. `arr-fill-and-fill-keys-repeat-one-value.mwlt`,
      `arr-from-drains-a-sequence.mwlt` and `arr-column-takes-one-cell-out-of-every-row.mwlt` exist.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — `mwl_stdlib::hash`'s
  module doc.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `orient.py` still has no selector for `docs/spec/01-core-library.md`, which every `arr` slice is
  specified by, and `[context] adrs` still wants `0007:5` for the next group's first item; this session
  worked off the members' own doc comments instead, which was enough but is not the spec.
