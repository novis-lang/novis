# Handoff

## State

**All three `Core\Decimal` slices landed**, as three `.nvst` cases over `divExact` and `divRound`. No
Rust change and no member change, so no new refcount edge and no valgrind run behind them.
`python tools/gaps.py` no longer lists `Core\Decimal` among the twenty-five thinnest classes; its two
members now carry eight cases between them.

- **Exactness is a bound on the quotient's *scale*, and the case names both sides of it** — `1 / 2^k`
  needs exactly `k` fractional digits, so one sweep of doubling divisors answers 28 times and refuses
  4 of 32, with `0.0000000037252902984619140625` beside the sentence refusing `1 / 536870912`. A
  second sweep by fives stops at the same `k` with a divisor eleven digits wider, which is what says
  the bound is the quotient's scale and not the magnitude of either operand.
- **The six modes partition by what the truncation discarded, counted** — all four discards (nothing,
  below half, above half, half) asked of all six modes, on both signs: `0 0 / 1 1 / 5 5 / 3 3`, 18 of
  48 rounding away. "Rounded away" is measured against `Down`'s own answer rather than recomputed,
  since `Down` is the truncation itself; the repeated column is `rounds_away`'s claim that away is
  from *zero*, one rule per mode rather than one per direction.
- **The two members agree wherever the division is exact, over a sweep** — `divRound` under every mode
  equals `divExact` 72 of 72 at the scale the quotient needs, and **0** of 72 one place short, which
  is what makes the first count non-vacuous. Where the quotient repeats the domains part outright: one
  refusal against six answers.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Secret` is the next two-member class whose whole surface is one rule and no dependency — no
store, no socket, no clock — and it is one file set: `crates/nvs-stdlib/src/secret.rs` and
`tests/conformance/core/`. Both members are the identity plus a `retain`, so every question below is
about what the *checker* admitted and what the run time did not touch; the module doc's own three
headings (why two names, the reason nobody reads, the reason need not be a literal) are the
specification, and ADR 0033 § 3 is the home. The existing cases are
`a-reveal-drops-the-secret-qualifier`, `reveal-is-the-identity-on-the-value`,
`a-revealed-value-is-accepted-by-every-secret-sink` and `reject/reveal-removes-secret-and-not-tainted`,
so none of the three below repeats a question already asked.**

- [ ] **`revealBytes` is the identity over octets no `string` can carry** — a sweep over the byte
      values, counted, with the revealed `bytes` equal to the original at every length including the
      empty one, so a member that answered through a `string` round-trip fails where a `string`
      cannot go. `crates/nvs-stdlib/src/secret.rs:152`.
- [ ] **The reason reaches no byte of the answer** — one secret revealed under a sweep of reasons of
      different lengths and contents, asserting all the answers **agree** rather than reading each,
      which is the module doc's "checked as a `string` and read by nobody" made a test.
      `crates/nvs-stdlib/src/secret.rs:133`, `crates/nvs-stdlib/src/secret.rs:152`.
- [ ] **The two members agree across the `string`/`bytes` boundary** — the same content revealed both
      ways, counted over a table, so the second name stays a name rather than a second rule.
      `crates/nvs-stdlib/src/secret.rs:133`, `crates/nvs-stdlib/src/secret.rs:152`.

## Backlog

- `Core\Http\Response` is the thinnest class `gaps.py` reports (status 3, text 3) — `docs/spec/01-core-library.md`.
- `Core\Totp` at depth 4.0 (check 4, code 4), and its window has no widening argument — ADR 0060 § 1.
- `Core\Cldr::pluralCategory` at depth 4.0 over a closed roster — ADR 0082 § 2.
- `Core\Task::afterResponse` is the one member with a PHP twin and no oracle case — `crates/nvs-stdlib/src/task.rs:561`, `tests/differential/`.
- A `reveal` in a loop is a refcount edge no conformance case can weigh; `crates/nvs-stdlib/tests/allocation_policy.rs` is where a leak would show.
- `Core\Cache::shared` and `Core\RateLimit::consume` stay blocked on the Docker daemon the plan's `Blocking` field names.
