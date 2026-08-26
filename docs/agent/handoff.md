# Handoff

## State

**Conformance is at 520 of 600, and it is the only frontier left.** The differential gate is met at
**159** of the 150 it requires and `python tools/gaps.py --differential` is empty. Verify is green
(1597 cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

**§ 12's `Core\Validate` leftover is closed and `Core\Uuid` is 5 cases for 5 members.** The first
names each character-class predicate's bound beside the entry past it — `~`/DEL for `isPrintable`,
DEL/`U+0080` for `isAscii`, `U+009F`/`U+00A0` for the C1 run — then sweeps `U+0000`..`U+010F` into
the four cells the two predicates carve out (95|33|32|112) and searches the same three bounds out as
run lengths (128|95|33), so a line that moved by one entry moves two counts. The second pins v7's one
behavioural claim: 64 consecutive draws compared by their 48-bit millisecond field, every adjacent
pair non-decreasing and the last also compared against the first, with the same sweep over `v4`
showing the counter can say no, and the field bounded on both sides between 2020 and 2100 so it is
the Unix clock rather than a monotonic sequence of its own.

**The old group's third slice is dropped, not carried**: `parse`/`tryParse` agreement over a table,
counted, is already on disk as `agree 8 of 8` in
`tests/conformance/core/uuid-every-draw-round-trips-and-a-refusal-quotes-boundedly.mwlt`.

## Next group

Three slices, all reading `crates/mwl-stdlib/src/math.rs` and adding new files under
`tests/conformance/core/`. `Core\Math` is 38 members over 13 cases — the widest members-per-case gap
left — and the 13 are listed by `ls tests/conformance/core/ | grep ^math`, which is worth one call
before writing, because the family cases are broad and each slice below is the claim they leave.
`docs/spec/01-core-library.md` § 3 owns the rules.

- [ ] **`isNan` and `isFinite` partition every float a member can answer** — the *agreement* shape:
      one table (`0.0`, `-0.0`, `Core\Math::EPSILON`, `INT_MAX as float`, `log(0)`, `sqrt(-1)`, a
      huge product) asked of both predicates, counting that exactly one of nan / finite / infinite
      holds per row, so a predicate that grew its own idea of infinity fails the count.
      `math.rs:285` (`isNan`), `math.rs:292` (`isFinite`).
- [ ] **`toBase` and `fromBase` are bounded on both sides at the base argument** — 2 and 36 accepted,
      1 and 37 refused, named together, with the refusal caught and its message asserted by prefix.
      `math-base-conversion-round-trips.mwlt` owns the round trip and asserts no bound.
      `math.rs:299` (`toBase`), `math.rs:306` (`fromBase`).
- [ ] **`hypot` agrees with `sqrt($x * $x + $y * $y)` everywhere the naive form is representable, and
      answers where it is not** — the *agreement* shape over a table, plus the overflow row that is
      the whole reason the member exists. `math.rs:159` (`hypot`), `math.rs:145` (`sqrt`).

## Backlog

- `Core\Csv` and `Core\Out` are the next-thinnest after `Core\Uuid` — 5 cases for 2 members and 3
  for 1 — `docs/spec/01-core-library.md` § 12.
- `Core\Json::decodeAs<T>`'s decoder reads a scalar-fielded class only (`mwl_stdlib::json` gap 2).
- ADR 0088's qualifier classification is on no `mwl-stdlib` member row (`mwl_stdlib::hash` doc).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
