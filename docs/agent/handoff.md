# Handoff

## State

**Goal 21 — the gaps no goal owned. Stage 2, the keystone, has landed whole**; stage 1's floor is
green and untouched. Every outstanding key in `crates/nvs-stdlib/tests/`'s four `*-outstanding.txt`
ratchets now carries `# <owner>` — a goal number from
[goals/chain.toml](goals/chain.toml) or the word `unowned` — and three tests in
`spec_registry_coverage.rs` hold it: the roster gate, the refusal on synthetic input, and the stale
direction. That file's own module doc, § *An outstanding key names its owner, in a column*, is the
design and says what the gate deliberately does not check (whether an owner is still *ahead* — the
chain holds the order, not the position).

**Twenty keys were seeded, not fifteen.** The item scoped this to the three `spec-*` lists;
`migration-members-outstanding.txt` is the same kind of file read by the same function, and a gate
over three of four ratchets would have been a half-gate. Two of the twenty came out `unowned` and
are bullets in [carried-gaps.md](carried-gaps.md) § *Unowned* under that file's own contract —
strike the owner, not the entry: `Core\Metrics` (goal 6 went green without registering the class)
and `Core\Process::spawn` (no chain entry builds it). Both are the user's to schedule.

Nothing is blocked. `python tools/chain.py --check` — stage 2's other check — passes, but it walks
the chain only; it does not read `carried-gaps.md`'s owner column, so the check's name promises more
than the tool does today.

## Next group

**Stage 3's first half: ADR 0067 § 3's `db.open` wildcard, decided in the goal's § *Standing
decisions* and needing no new decision.** One file set: `crates/nvs-config/src/capability.rs` and
its own `#[cfg(test)]` module at the foot of the same file.

- [ ] **A `*.` entry matches a host at a label boundary** — `crates/nvs-config/src/capability.rs:269`
      is `host_granted`, the single home for the comparison (both `allows` at
      `crates/nvs-config/src/capability.rs:281` and `allows_host` go through it, and ADR 0057 § 4
      forbids the two disagreeing). Case-insensitive, and `*.tenants.internal` matches
      `a.b.tenants.internal` and not `tenants.internal` or `evil-tenants.internal`. The goal names
      the tests: `a_wildcard_grant_matches_a_subdomain_at_a_label_boundary`,
      `a_wildcard_grant_does_not_match_the_bare_domain`,
      `a_wildcard_grant_does_not_match_a_suffix_inside_a_label`.
- [ ] **A bare `*` is refused as a second spelling of every host** — `Grant::Everything` already
      means it, and R20 forbids the second way in. `crates/nvs-config/src/capability.rs:253` is
      `grant_of`, which turns a `Setting` into a `Grant`. Test:
      `a_bare_star_is_refused_as_a_second_spelling_of_every_host`.
- [ ] **`net.connect` takes no wildcard** — it is asked of a *resolved address*
      (`crates/nvs-config/src/capability.rs:43` is `Cap::NetConnect`, and the comment at
      `crates/nvs-config/src/capability.rs:85` says why the question arrives already resolved), so
      there is no name left to match. Test:
      `net_connect_takes_no_wildcard_because_it_is_asked_of_an_address`.

Strike `carried-gaps.md`'s first *Owned* row with the first slice, and the wildcard's line in
`crates/nvs-config/src/capability.rs`'s own § *Known gaps*.

## Backlog

- Stage 3's second half — `nvs check` resolves `nvs.toml` as `nvs run` does; `crates/nvs-types/src/intrinsics.rs`
  gap 6 (checking has no configuration in front of it), three `-p nvs-cli` tests the goal names.
- `tools/chain.py --check` does not read `carried-gaps.md`; stage 2's second check reads as though
  it does. Either the tool gains the walk or the check's name is narrowed.
- Goal 27 is the same column one level down — ~110 module-doc `# Known gaps` items with no owner
  ([carried-gaps.md](carried-gaps.md), last *Owned* rows).
