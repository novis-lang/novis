# Handoff

## State

**Stage 8's differential floor is met: the suite holds 210 passing cases against the acceptance
check's 210.** The four cases this session added are all `Core\Math`, all in
`tests/differential/core/`, and all pass on both legs — PHP 8.5.9 is on `PATH` here and computes
every expectation, so nothing is frozen by hand.

**The item's premise was half wrong and the group was re-aimed.** `Core\Math::lcm` and
`Core\Math::hypot` already had oracle cases; their depth floor of 3 is `gaps.py`'s *conformance*
figure, which the differential count does not read. The three replacement cases went to members
with no differential case of their own — `ceil`/`floor`/`truncate`, the six hyperbolics, and
`asin`/`acos`/`atan` — each asking the boundary rather than another in-domain row. The playbook
bullet owns the trap.

**`Core\Math::fdiv` has its oracle case**, which was the one member `gaps.py` named. It pins every
row the `/` operator will not answer: a zero divisor in both its signs, a zero over a zero, either
operand infinite or `NAN`, a quotient that overflows, and — asserted rather than described — that
`/` refuses the zero divisor in *both* languages while `fdiv` is what each offers instead.

**A real divergence turned up and is not pinned anywhere.** PHP's `round()` pre-rounds against the
decimal spelling, so `round(0.285, 2)` is `0.29` and `round(1.005, 2)` is `1.01` in PHP where
`Core\Math::round` answers `0.28` and `1` — neither value is a tie in binary, and
`tests/conformance/core/math-format-and-round-agree-wherever-both-name-the-same-precision.nvst:21`
already states that as Novis's deliberate position. It is a decided divergence with no divergence
case and no line in the reference, which is the shape `math-min-and-max-diverge-from-php-…` has.
It is the first item of the next group.

**`orient.py`'s `[context]` gaps.** Carried forward, all still unfixed: no field selects
`docs/reference/lang/*.md`; `adrs` does not name ADR 0125 § 2 or ADR 0066 § 3; `modules` does not
name `crates/nvs-ir/src/lower/operator.rs`, `crates/nvs-ir/src/ty.rs`, `docs/adr/README.md` or
`ground-rules.md`; the pack prints the goal item but not the `[[check]]` grading it; and `modules`
still names `crates/nvs-host/src/budget.rs`, which matches nothing. New this session: nothing
selects `tests/differential/` or `tools/gaps.py`'s own output, so an item about test coverage
arrives with no way to see what the suite already holds.

## Next group

**`Core\Math::round`'s divergence from PHP, then the last member with a PHP twin and no oracle
case. File set `tests/differential/core/`, `crates/nvs-stdlib/src/math.rs` and
`crates/nvs-stdlib/src/task.rs`.**

- [ ] **The `round` divergence gets its pinned case.** A `--ORACLE--` case that *asserts* the
      difference the way `math-min-and-max-diverge-from-php-over-a-tie-a-numeral-string-or-an-unordered-pair.nvst`
      does — both languages compute, and what is printed is each side's own answer folded to a
      shared rendering, so the case passes while naming the gap. The body is at
      `crates/nvs-stdlib/src/math.rs:1765` and the decision it implements is stated at
      `tests/conformance/core/math-format-and-round-agree-wherever-both-name-the-same-precision.nvst:21`.
- [ ] **The same divergence gets its line in the reference.** Wherever the `Core\Math::round` card
      is rendered — the row is at `crates/nvs-stdlib/src/math.rs:134` and the card block follows
      the class — plus the aggregated divergence list, so a reader porting PHP finds it before the
      test does.
- [ ] **`Core\Task::afterResponse` gets its oracle case.** The last member `gaps.py`'s
      *differential gap* block names, twin `fastcgi_finish_request`, at
      `crates/nvs-stdlib/src/task.rs:561`. Check first that the CLI leg can run it at all — if the
      twin has no meaning outside a request, say so in the case's `--TEST--` and cover the
      observable half.

## Backlog

- `$cls::f(...)` as ADR 0027's first-class callable still records `ExprInfo::CallableRef`, so the
  `Closure` names `T`'s method rather than the implementor's — `crates/nvs-ir`'s module doc.
- A dynamic `instanceof` narrows nothing; `crates/nvs-types/src/locals.rs:481` narrows only off
  `ExprInfo::InstanceOf { class }` and nothing in the reference claims either way.
- `Core\Script::args` is the thinnest member in the thinnest class at depth 3, body at
  `crates/nvs-stdlib/src/script.rs:316` — `docs/agent/loop-goal.md` Stage 8.
- 80 unasserted error paths remain, 78 of them `Fault::fatal` — `python tools/gaps.py` ranks them.
- Migration corpus sits at 37% over its 36% floor — `docs/plan/m6.md`.
