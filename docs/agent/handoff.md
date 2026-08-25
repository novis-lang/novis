# Handoff

## State

**Stage 0 items 1, 2, 3, 4 and 9 are done; item 5 is in progress — 5a and 5b landed, 5c is next.**
ADR 0047 § 4's *assignability* half now runs. `is_assignable`
([expr/assign.rs:41](../../crates/mwl-types/src/expr/assign.rs#L41)) takes the interner by `&mut` and
answers all four free-widening rows with one recursion through `TypeInterner::literal_base`, so the rows
compose with everything below them for free — `"a"` reaches `string|null`, `tainted string` and a shape
field with no row of its own. The reverse direction needs no rule to refuse it: nothing widens a base type
downwards, and § 4's last three rows are a checked `as`.

The producer half is the placement rule, not a second table: a literal expression takes its singleton type
from the position it lands in (`placed_literal`,
[expr/literals.rs:50](../../crates/mwl-types/src/expr/literals.rs#L50)) and its plain base everywhere
else, which is exactly what `uint` and `decimal` placement already do in that module. `expr::members`
reaches the same helper for § 3's enum case, so `Mode::Read|Mode::Write $m = Mode::Read;` checks while a
bare `int` still does not. `-1` needed a pair of its own — the atom carries its sign in type position but
a `-1` *expression* is a negation wrapping `1` — so `negated_literal_expectation`/`negated_literal_result`
push the placement through the operator and put the sign back.

`python tools/verify.py` is green (1345 tests). No valgrind run: no new refcount edge, and § 5 gives these
types no representation of their own.

**Not yet true, and expected:** `as` does not place a *string* literal operand at its target
([operators.rs:56](../../crates/mwl-types/src/expr/operators.rs#L56) places only `Int`/`Float`), no
diagnostic refuses a statically-impossible conversion, and nothing lowers the runtime membership test — 5c
and 5d.

## Next group — ADR 0047 § 4's checked `as`, § 6's diagnostics, and the runtime half (Stage 0 item 5)

**Shared file set:** `crates/mwl-types/src/expr/operators.rs` and `crates/mwl-diagnostics/src/lib.rs` for
5c, then `crates/mwl-types/tests/literal_types.rs` and `tests/conformance/lang/` for 5c/5d.

- [ ] **5c — § 4's checked `as`, and § 6's two diagnostics.** `infer_conversion`
      ([operators.rs:48](../../crates/mwl-types/src/expr/operators.rs#L48)) gains the `base`/`mixed` →
      literal and enum → case-subset rows. Two parts: extend the placing branch at
      [operators.rs:56](../../crates/mwl-types/src/expr/operators.rs#L56) from `Int`/`Float` to `Str`, so
      `"a" as "a"|"b"` is statically satisfied rather than converting a plain `string`; and refuse a
      statically-impossible conversion where the operand's own type proves it (`"z" as "a"|"b"`,
      `Mode::Admin as Mode::Read|Mode::Write`). § 6 names both diagnostics and states the message shape —
      the accepted set is generated from the type, never written per site. Next free code is **E0469**
      (`python tools/brief.py` re-derives it; never reuse a retired number).
- [ ] **5d — the runtime half.** A conversion whose operand is only known at run time (`mixed`, or a
      wider union) needs § 5's membership test lowered in `mwl_ir::lower`, beside the `uint`/enum
      conversion it is a further-restricted version of — the target erases to its base
      (`lower_checked_ty`), so the *set* has to travel with the check. Then a `.mwlt` case under
      `tests/conformance/lang/` pinning that the conversion throws off the happy path and that a
      literal-typed binding costs nothing extra when it does not.

## Backlog

- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — plan, `Open now`.
- `private`/`protected` enforcement (Stage 0 item 6) — `docs/agent/loop-goal.md` § *Stage 0*.
- `Comparable`/`Stringable` member signatures (item 7) — same file.
- ADR 0061's `autoload` grammar and name-to-file fixpoint (item 8) — same file.
- `equality_domain` puts a literal type in its base's ADR 0090 domain, so `$mode == "z"` compares two
  strings; refusing non-overlapping literal *sets* would be a new row in ADR 0090 § 2.
- `Core` breadth resumes at `examples/collect.mwl` — plan, `Open now`.

`orient.py` printed everything this session needed. One gap worth noting for the manifest: `[context]
modules` does not select `mwl-types/src/expr/quals.rs`'s or `ty.rs`'s *item* docs, only the module line,
and both were read to settle where the widening belonged.
