# Handoff

## State

**M4's Stage 8, depth.** The tree is at **817 conformance plus 189 differential**. Nothing is
blocked.

`Core\Random` is done as a depth target, and it took two cases neither of which is the work the
previous handoff named. All three items of that group were **already asserted** — see the new
playbook bullet under *Writing a test case*, which is the reusable half. What was actually
missing was found by reading the member doc comments for a rule no existing case could observe,
every one of them drawing over a *list*:

- **The subject's keys reach no answer**, asked of `pick`, `sample` and `shuffle` together over a
  map whose keys and values are disjoint strings. `pick` draws a value and never a key (150 draws,
  counted both ways, all three values reached) — the `array_rand` divergence `random::pick`'s doc
  comment owns; both array answers come back a list under fresh `0, 1, …` keys, rendered as well
  as counted.
- **`sample`'s order is itself drawn**, the other `array_rand` divergence, which no count of
  entries can see. A two-entry sample puts the lower-positioned entry first in *some* of 40 draws
  and not all — a preserving implementation gives 40 and a reversing one 0, so it is a bound on
  both sides. Plus the full-width draw as a permutation, and the two widths with no order at all.

Both cases are probabilistic by construction and were re-run eight times before the wrap; the
failure probabilities are stated in the case comments (the tightest is 2^-40).

The gap seven handoffs back still stands: **no `Core` class reaches `nvs_hir::implements_interface`**,
so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog
and still deserves a session of its own.

## Next group

**`Core\Random`'s remaining unasserted rule, then the next class `gaps.py` ranks** — one file set:
`crates/nvs-stdlib/src/random.rs` and `tests/conformance/core/`. Read the existing cases' bodies
before writing, per the new playbook bullet.

- [ ] **A drawn entry is the subject's own value, not a copy of it**
      (`crates/nvs-stdlib/src/random.rs:220` — `owned_value_at`'s retain) — one case, the
      *invariant* shape. Draw with `pick` and `shuffle` over an `array<mixed>` of objects and
      assert identity against the subject's entries (ADR 0090 makes `==` over two objects
      identity), which is what the slot-based draw buys and what a copying implementation loses.
      Nothing in `Core\Random`'s seven cases uses a non-scalar element at all.
- [ ] **Take `gaps.py`'s next-thinnest class after re-running it** — the rank moves once these two
      cases land, and the bullet above is the reason not to trust the previous rank's group
      unread.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`; `$a < $b` over two `Core\Uri`s is
  `E0411` (ADR 0013, and the plan's *Open now*).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `Core\Random\Seeded` is unbuilt (`random.rs` module doc, known gap 1).
- `ThreadRng` is not reseeded on `fork` (`random.rs` module doc, known gap 2).
