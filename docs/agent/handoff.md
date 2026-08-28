# Handoff

## State

**M4's Stage 8, and `Core\ObjectSet` is closed**: both slices were pure test cases over
landed work, so no `crates/nvs-stdlib/src` line changed, and the class went 0.89 →
**1.11** (10 cases over 9 members). The tree is at **759 conformance plus 189
differential**. `python tools/loop.py --list` still reports no named `.nvst` case owed by
any stage, and `gaps.py --differential` still ranks 0. `Core\Time\Duration` (0.95) is now
the thinnest class and `Core\Math` (0.97) the next.

- **The algebra is degenerate at both of its own ends**, over four receivers — empty, a
  singleton, three distinct members, and two equal-looking `Tag`s — with ten laws each,
  asserted by counting the 40 agreements. Union and intersect with the receiver itself
  are that receiver in *members* and not only in size; `diff` from itself and `intersect`
  with a disjoint set are the two annihilators; a disjoint operand is `diff`'s identity
  and adds its whole self to a `union`. The twins row carries the identity rule (ADR 0090
  § 4): two `Tag("red")`s are two members and a third is neither of them.
- **The six mutators are one invariant.** After every step of a scripted run —
  a duplicate `add`, a `remove` of a value never held, a `clear` of an already-empty set,
  an `add` after a `clear` — the count, the length of the `foreach`, the membership sweep
  over the universe and `isEmpty` are asked together and must agree; 12 steps, 12
  agreements. `clear` is then shown to be exactly `remove` over every element by emptying
  two identical sets the two ways, and insertion order after a `clear` starts over.

**An algebra result is unparameterized** — `union`/`intersect`/`diff` return
`CoreTy::Instance(NAME)` with no type argument, so it cannot be bound or passed anywhere.
The playbook § *Writing a test case* holds the spelling that works; the fix is a registry
one and is in the backlog below.

## Next group

**`Core\Time\Duration` is the thinnest class left** (0.95, 18 cases over 19 members) and
the file set is `crates/nvs-stdlib/src/time.rs` plus `tests/conformance/core/`.
`DURATION`'s block starts at `time.rs:130`, so the first two slices share it; the third is
`Core\Math` and a different file.

- [ ] **`Core\Time\Duration`'s one uncased member** (`crates/nvs-stdlib/src/time.rs:130`,
      `DURATION`'s block) — spec § 4. `python tools/gaps.py --member 'Core\Time\Duration'`
      lists the seven cases that already exist; the member no case calls is what this
      slice writes, and the shape to reach for is the boundary the member is written
      around rather than another row of an existing table.
- [ ] **`Duration`'s arithmetic at the ends of its own storage**
      (`crates/nvs-stdlib/src/time.rs:130`, `DURATION`'s block) — spec § 4, ADR 0070 § 1.
      The nanosecond scale is an `i64`, so a `plus`/`minus` that carries past it has a
      first refused value and a last accepted one; name them together, and assert the
      refusal is a catchable `Throwable` naming the member.
- [ ] **`Core\Math`'s one uncased member** (`crates/nvs-stdlib/src/math.rs`) — 0.97, 37
      cases over 38 members. `gaps.py --member 'Core\Math'` names it; a different file set
      from the two above.

## Backlog

- `Core\ObjectSet`/`Core\ObjectMap`'s `union`/`intersect`/`diff` (and every member
  returning `CoreTy::Instance`) erase their type argument, so the result is
  unbindable — `E0401` printing both sides as `Core\ObjectSet`. Two bugs in one, exactly
  as the `Core\Unit` entry below: the erasure, and a diagnostic that renders the two sides
  identically. The fix belongs to `nvs_stdlib::registry` plus `nvs_types`.
- A `Core` enum cannot be swept from an `array<Core\Unit>` or passed through a
  user-declared `Core\Unit` parameter — `E0401`, rendered *"expected `Core\Unit`, found
  `Core\Unit`"*. `docs/agent/playbook.md` § *Writing a test case* has the workaround; the
  fix belongs to `nvs_types`.
- 54 of the 156 guard tests `loop-goal.toml` names still match nothing `cargo test` would
  run — `docs/agent/guard-name-debt.md`.
- `Core\Uri` and `Core\Time\DateTime` are the classes just above the frontier at 1.00.
