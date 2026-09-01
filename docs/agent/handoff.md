# Handoff

## State

**All three `Core\RateLimit` local-tier slices landed**, as three `.nvst` cases over `shed` and the
`Decision` readers. No Rust change and no member change, so no new refcount edge and no valgrind run
behind them. `python tools/gaps.py` now reads `Core\RateLimit` at depth 4.5 with `consume 3, shed 6`:
the class's remaining thinness is entirely `consume`, which is the Redis half and waits on the Docker
daemon the plan's `Blocking` field names.

- **The window is bounded on both sides and the two arrivals are named together** — twelve arrivals
  under one limit of eight, with `admitted 8 and shed 4 of 12` beside `last admitted 8, first shed 9`.
  A limiter that spent its budget one arrival early prints the same word on the seventh line and the
  ninth as a correct one does; only the pair of counts fails for it. Those two numbers also say the
  window does not reopen, since an admission at the eleventh would move one without the other.
- **The three readers are asked after every arrival of two windows of different widths** — three an
  hour and five — and made to *agree* rather than read right: `limit` answers its own argument 14 of
  14, `remaining` falls by exactly one per admitted unit and then holds at zero, and `retryAfter` is
  absent exactly when allowed and otherwise the window's own drain interval (1200s and 720s), never
  the rest of the period. A refusal charges nothing, so the wait does not push itself further out.
- **`shed` keys its own memory** — two keys taken alternately under one limit of four admit 8 of 8 and
  agree on what is left at every arrival, where a single shared arrival time would answer 3 and 2 at
  the first pair; the fifth on each is shed, and a third key opens on a full budget.

**One unrelated fix rode along**: `nvs-host`'s `a_read_past_its_deadline_reports_a_timeout` failed the
full gate and passed alone, because it measured the wait from an `Instant::now()` taken inside the
spawned task rather than from the deadline the socket was given — the scheduler's start latency was
being subtracted from the interval under test. It now compares against the deadline itself. The
playbook bullet is the general shape.

**The two `orient.py` warnings are still there**: `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Decimal` is the thinnest class left whose whole surface is arithmetic — no store, no socket,
no terminal — and it is one file set: `crates/nvs-stdlib/src/decimal.rs` and
`tests/conformance/core/`. It is also a two-member class, so the three slices below are the same two
bodies asked three ways. ADR 0054 § 3 is the home of why the named-rounding members exist at all: a
`decimal` division is the one place rounding is business logic rather than an artifact of the
operator, so the mode is an argument and never a default. `divExact`'s refusal and `divRound`'s mode
are inside 40 lines of each other.**

- [ ] **`Core\Decimal::divExact` is bounded on both sides of exactness** — the last quotient it
      returns and the first it refuses over one sweep of divisors, named together and counted, so a
      member that tested the wrong remainder prints plausibly against either half alone.
      `crates/nvs-stdlib/src/decimal.rs:204`.
- [ ] **Every rounding mode answers the same tie and the same non-tie, counted** — one quotient asked
      of the whole mode roster, asserting the modes are pairwise distinct where they must differ and
      identical where they must agree, so a mode that grew its own arithmetic fails here while its own
      line still reads right. `crates/nvs-stdlib/src/decimal.rs:235`.
- [ ] **The two members agree wherever the division is exact** — `divRound` under every mode equals
      `divExact` for a divisor that divides, and only `divExact` refuses when it does not, which is
      the agreement neither member's own case can state. `crates/nvs-stdlib/src/decimal.rs:204`,
      `crates/nvs-stdlib/src/decimal.rs:235`.

## Backlog

- `Core\RateLimit::consume` stays at 3 cases until a Docker daemon is reachable — plan, `Blocking`.
- `Core\Secret` at 3.5 (`revealBytes` 3, `reveal` 4) needs no service either — ADR 0033 § 3.
- `Core\Cache::local` at 4 cases is reachable with no store; `shared` is not — ADR 0059 § 1.
- `Core\Task::afterResponse` is the only differential gap left, against `fastcgi_finish_request`.
- `Core\Cldr::pluralCategory` at 4 over a closed roster — ADR 0082 § 2.
- The `[context] modules` matcher drops two patterns it then prints — `docs/agent/loop-goal.toml`.
