# Handoff

## State

**M4, and the frontier moved: Stage 00 of [loop-goal.md](loop-goal.md) is now the first thing the
acceptance test asks for, and it is red on its merits.** `python tools/loop.py --goal-only` stops at
`nvs-syntax (the for header)` naming `a_for_init_clause_declares_one_typed_local`, which is the first
test to write. `python tools/verify.py` is green at 1758 Rust tests, 868 conformance and 189
differential, and stays the gate on committing; the goal being red is the worklist, not a breakage.

An audit pass the user fired by hand probed the surface rather than the refusal sites, and found six
shapes a program reaches for on its first page that either do not parse or panic the compiler. They are
Stage 00, ahead of Stage 0 and Stage 0a. The one that moves the most files is item 44: **ADR 0109 is
Accepted**, folded into ADR 0007 § 1's binding-site table, and its corpus migration is half the item
rather than a follow-up.

Two blind spots in the gate itself explain why these were not already scheduled, and both are worth
knowing before trusting a count in this repository:

- `refusals.rs` counts **arms**, and one catch-all arm is one number however many shapes fall into it.
  `crates/nvs-ir/tests/type_atoms.rs` is the answer: a table of *shapes*, each asserted to reach a
  diagnostic or an IR, with a `KNOWN_ICE` ratchet that may not grow.
- `holes.py` recognizes a refusal by **phrasing** — three house sentences. Item 45's panic uses none of
  them, so it is not among the four sites either tool reports. Item 49 widens both, and re-derives
  `CEILING` from a count that is not blind.

## Next group

**Item 44 — the `for` header and the corpus that moves with it.** File set:
`crates/nvs-syntax/src/parser/stmt.rs`, `crates/nvs-syntax/src/parser/tests/stmt.rs`,
`crates/nvs-diagnostics/src/lib.rs`, then every `for` in `tests/conformance/`, `tests/differential/`
and `examples/`. Read it with `python tools/holes.py --item 44`, and
[ADR 0109](../adr/0109-a-for-header-declares-its-own-counter.md) is the rule.

- [ ] **`parse_for`'s init clause takes a declaration or an expression list** (`stmt.rs:321`, § 1).
      A checkpointed trial parse, the same technique `array<T>` in expression position already uses —
      a header whose init begins with a `Name` token is either until the `$` after it settles which.
      The other two clauses do not change.
- [ ] **`E0124` and `E0125`** (§ 3), on a mixed init clause and on two declarations in one. Next free
      in the parser band is `E0124`, so the two are consecutive. The checker and `nvs-ir` are owed
      nothing (§ 4).
- [ ] **Migrate the corpus.** Every `for` whose counter is declared on the line above and reassigned in
      the header takes the declaration into the header. No expected output moves — the counter is
      function-scoped either way (§ 2) — so **the suites staying green is the review**. A counter read
      after its loop, or shared by two loops, stays declared above.
- [ ] **The four named guards and the case.** `loop-goal.toml`'s first block names them; delete each
      bullet from [guard-name-debt.md](guard-name-debt.md) § *Stage 00's ten* as it lands.

## Backlog

Stage 00's other five, in the order they are written there. Items 45-48 are one file set each and none
of them touches the others:

- **Item 45** — a user-declared class constant: `mixed` at the checker, a panic at
  `crates/nvs-ir/src/lower/expr.rs:262`. ADR 0011 makes this the only way to spell a constant.
- **Item 46** — `Base::make(): static` called as `Leaf::make()` types as `Base`, which
  [docs/plan/m4.md](../plan/m4.md)'s own acceptance paragraph says it must not.
- **Item 47** — `instanceof` narrows to a class and not to an interface. Beware the near-twin test name;
  guard-name-debt § *Stage 00's ten* says why.
- **Item 48** — `new Core\Error("x")` panics where the same shape on a user class is `E0402`.
- **Item 50** — PHP's `case Name = 1;` enum body cascades to four diagnostics where `trait` and
  `(int)$x` each give one naming their ADR.
- **Item 49 last**, because it is written against what the rest close.

Then Stage 0 as before. Beyond the goal:

- Item 16's `callable $g = $f(...);` (`lower/call.rs:773`) and item 25's three shapes are unchanged and
  still the only entries in `KNOWN_ICE`/the refusal ceiling.
- **`website/` has 8 stale pages** (`cd website && python site.py check`). Not a gate by design.
- **At the M4 → M4B boundary, not before:** re-run `python tools/playbook.py --goal --min 3`, re-measure
  with `loop-stats.py`, and only then revisit the 120k slice gate in AGENTS.md.
