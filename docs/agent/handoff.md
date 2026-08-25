# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 12, 16 and 17 are done**, so what is open is **items 13, 14 and 15**, one
or three named tests each at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py`
short-circuits at stage 0, so **Stage 3 is shut until those three clear**.

**Item 12 landed, and it was a deletion after all** — both `Core\Uri::isValid` and
`Core\Uuid::isValid` existed, against what the previous handoff said; the playbook's new *Tooling*
bullet says how that claim got made. `$s as ?Core\Uri` and `$s as ?Core\Uuid` now type-check *and*
lower: `mwl_stdlib::registry::PARSE_ROSTER` names one non-member symbol per roster class,
`mwl_types::expr_table::ExprInfo::ParseRosterConversion` carries it to `mwl-ir`, and every other
class target is `E0473`. ADR 0066 § 3 carries the one amendment the deletion needed — `Uri::isValid`
asked "is this an **absolute** URI", one condition more than `!= null`, and that condition is now
`($s as ?Uri)?->scheme() != null`.

Verify is green (1549 tests, 73 suites, clippy and fmt clean); valgrind clean on the roster
conversion's new refcount edge. Conformance **434**, differential 89.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, genuinely blocked behind ADR 0088's sink
carriers; it is M4S work, not a slice to open ahead of the sinks.

## Next group — item 13, then 14, then 15

**These three share no file with each other**, so a session takes exactly one and stops; the second
slice the ceiling allows has nothing cheap to be. Item 15 is by far the largest (a second array
representation with a degrade path, ~300–500 lines across `array.rs` and the ABI).

- [ ] **Item 13 — `Core\Uri` compares by normalized components**, with a `parse` → serialize →
      `parse` round-trip property test. Tests: `two_uris_compare_by_normalized_components`,
      `a_parsed_uri_round_trips_through_its_own_text`, `-p mwl-stdlib`. RFC 3986 § 6.2.2 is the
      normalization; `parse` still reports verbatim, because the normalizing happens at the
      comparison. Anchors: `crates/mwl-stdlib/src/uri.rs:453` (the eight slot constants the
      comparison reads), `crates/mwl-stdlib/src/uri.rs:287` (`CLASS`), and
      `crates/mwl-runtime/src/identity.rs:137` (`value_identical`, which is what `==` over two
      `Core` instances reaches today and answers by identity — `mwl-stdlib`'s `uuid` gap 2 is the
      same hole seen from the other side). Add the `fuzz/fuzz_targets` entry beside `lex.rs`.
- [ ] **Item 14 — a call-stack limit rides the safepoint's emit site** (ADR 0020 § 1). Tests:
      `a_function_entry_checks_the_stack_limit`, `a_leaf_function_under_the_slack_emits_no_stack_check`,
      `a_runaway_recursion_reports_a_limit_rather_than_faulting`, `-p mwl-codegen`. Anchors:
      `crates/mwl-codegen/src/emit.rs:758` (`emit_safepoint`, the site to ride),
      `crates/mwl-codegen/src/emit.rs:370` (`InstKind::Safepoint`'s dispatch).
- [ ] **Item 15 — a second array representation with a degrade path**, plus the `php_ratio`
      benchmark material. Tests: `a_list_shaped_array_holds_no_index_map`,
      `an_integer_subscript_allocates_no_key`, `a_non_sequential_key_degrades_the_packed_array`,
      `both_representations_answer_every_primitive_alike`, `-p mwl-runtime`. Anchor:
      `crates/mwl-runtime/src/array.rs:362` (`MwlArray`). `benches/userland/` and `tools/bench.py`
      are this item's `php_ratio` material and are **committed now** — a concurrent session landed
      them in `996d28a` — so what is left is measuring against them. `Cargo.toml`'s `exclude` line
      is what keeps that directory from breaking the workspace; leave it.

## Backlog

- `Core\Out::capture`, the last `§12` key in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`
  — M4S sink work, `docs/implementation-plan.md` § *Open now*.
- ADR 0066 § 3's other two refusals (`$i as ?string` cannot fail; `$arr as ?int` does not exist) —
  `mwl-ir`'s crate-doc gap 4 names both; each still panics in lowering rather than diagnosing.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2.
- `do`/`while` does not lower — `mwl-ir`'s crate-doc control-flow gap.
- ADR 0088's registry-wide qualifier classification for every `Core` `string`/`bytes` parameter —
  `docs/implementation-plan.md` § *Open now*.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
