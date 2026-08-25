# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 14, 16 and 17 are done**, so what is open is **item 15 alone**, four
named tests at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py` short-circuits
at stage 0, so **Stage 3 is shut until it clears**.

**Item 12 landed a second time, as a reversal the user asked for.** ADR 0066 § 3's two-class
**parse roster** — `$s as ?Core\Uri`, `$s as ?Core\Uuid` — is **withdrawn**. `as` now targets no
class at all: every class or interface target is `E0473`, with no roster to consult. The
non-throwing parse is `Core\Uri::tryParse(string): ?Uri` and `Core\Uuid::tryParse(string): ?Uuid`,
ordinary member rows over each class's own `parse`, and ADR 0063 R5's `try…` ban gained its one
exception (new § 3a, with three conditions) to admit exactly that spelling.

Why, in one line, because the ADR argues it in full: `as?` spells a *downcast* in every language a
reader arrives from, so spelling a **parse** that way inverted the syntax's one intuition — and it
did so for exactly two class names a reader had to have memorized. R17 was said to forbid a second
spelling of `parse`, but `Core\Uri::parse($s)` and `$s as ?Uri` both existed: the roster never
removed a spelling, it relocated one from a member name to an operator, and to the harder of the
two to read.

**Both `isValid` deletions stand** — that half of item 12 was right, and CVE-2024-5458 is still the
argument. `Uri::isValid`'s extra condition is now `Core\Uri::tryParse($s)?->scheme() != null`.

Deleted with the roster: `registry::PARSE_ROSTER`, `parse_roster_symbol`,
`ExprInfo::ParseRosterConversion`, its `mwl-ir` lowering arm and `Lowering::parse_roster_symbol`,
and the `symbols()` chain that carried two non-member symbols. `lower_conversion` lost its `span`
parameter with them. `E0473` **stayed and got simpler** — it fires unconditionally now, and names
`tryParse` in the help for a class in the new `registry::TRY_PARSE_CLASSES`, which drives nothing at
run time and exists only so the registry tests can hold § 3a's conditions.

Verify is green (1555 tests, 74 suites, clippy and fmt clean). Conformance **435**, differential
**90** — unchanged; the two `.mwlt` cases were rewritten to the surviving spelling rather than
added to. `mwl test tests/` reports 519 passed / 6 failed, the same six PHP-on-Windows oracle
failures as before.

**One inconsistency found and deliberately not fixed.** [spec § 13](../spec/01-core-library.md)
says `isBoolean`'s replacement is `$s as ?bool != null`, but ADR 0035 makes `as bool` **total**, so
`as ?bool` is a § 3 "conversion cannot fail" compile error. `isInteger`/`isFloat` are fine. Deciding
what `isBoolean` actually becomes is a design call, not a typo fix, so it is left for the user —
neither spelling is built yet, so nothing on disk is wrong today.

## Next group — item 15, three slices over one file

Unchanged by this session. They share **`crates/mwl-runtime/src/array.rs`** and its own
`#[cfg(test)]` module; nothing else is touched until the last one. [loop-goal.md](loop-goal.md)
item 15 is the specification, and that file's module doc owns the decision, the PHP comparison and
what the ABI addition costs if it waits. [ADR 0007 § 5](../adr/0007-explicit-type-system.md) is
**unchanged** — this is representation, not semantics.

- [ ] **The packed representation itself** — a list-shaped array holds no index map and no key
      strings. Test `a_list_shaped_array_holds_no_index_map`, `-p mwl-runtime`. Anchors:
      `crates/mwl-runtime/src/array.rs:327` (`ArrayHeader`), `:362` (`MwlArray`).
- [ ] **The integer subscript path** — reaching an element by integer index allocates no key. Test
      `an_integer_subscript_allocates_no_key`. Anchors: `crates/mwl-runtime/src/array.rs:795`
      (`mwl_array_get`), `:846` (`mwl_array_set`), `:880` (`mwl_array_append`).
- [ ] **The degrade path and the equivalence** — the first key that breaks the invariant falls back
      to today's hash form with no observable difference. Tests
      `a_non_sequential_key_degrades_the_packed_array`,
      `both_representations_answer_every_primitive_alike`. Anchors:
      `crates/mwl-runtime/src/array.rs:966` (`mwl_array_next_slot`), and the
      `mwl_array_key_at`/`mwl_array_value_at` pair the `foreach` cursor reads through.
- [ ] Last, and only after the three above are green: the **first** `docs/perf/history.ndjson`
      entry, with a `php_ratio`, per [ADR 0026](../adr/0026-performance-measurement-methodology.md).
      That file not existing is why nothing caught this.

## Backlog

- `Core\Fatal::onLimit` and the `[limits] fatal_reserve_*` directives — ADR 0020 § 1, M4S/M7.
- Spec § 13's `isBoolean` replacement, above — `$s as ?bool` does not exist.
