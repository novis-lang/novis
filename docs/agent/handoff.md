# Handoff

## State

**Goal `gap-zero`, stage 2, and the run is held.** The tool half is landed: `tools/owners.py` has two
owner kinds — a live goal slug and an M9+ milestone tag — `unowned` is refused by name, `--check` is
the whole gate (untagged, broken, `unowned`, past-milestone, goal-owned and retired-owner all fatal,
its three flags accepted and doing nothing for the carried floor), and `tools/verify.py` runs it as
step 5 of 12. `python tools/owners.py --check` reads five zeros over 24 milestone-owned gaps.

**The ratchet half and stage 3 are blocked on one decision.**
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:24` holds six keys tagged
`unowned` — `Core\BigInt` and `Core\Test`'s five double members. No goal on the chain builds them and
no M9+ plan file states their scope, so `owner_problem` cannot refuse `unowned` without turning
`every_outstanding_key_names_an_owner` red, and `docs/agent/carried-gaps.md` cannot be deleted while
§ *Unowned* holds their reasons and that file's own header points at it. Goal `gap-zero`
§ *Standing decisions* forbids inventing an owner, so the tree is left green and the run holds.

## Next group

**Stage 2's ratchet half and stage 3, once the six keys have an owner** — one file set:
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`,
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`, `docs/agent/carried-gaps.md`,
`tools/playbook.py`, `tools/brief.py`.

- [ ] **Re-owner the six keys as the user decided, then make `owner_problem` refuse `unowned` and
      name the guard `unowned_is_no_longer_an_owner_for_a_key`** —
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:505` is the function,
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:546` the test that calls it, and
      `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:19` the header paragraph
      that explains the column. `rule:core-api/tier-roster` is what places `Core\BigInt`.
- [ ] **Delete `docs/agent/carried-gaps.md` and re-point every reader in the same slice** —
      `tools/playbook.py:130` (`CARRIED_GAPS`, and `:149`, `:452`, `:462`, `:508`, `:567`),
      `tools/brief.py:472` and `tools/brief.py:483` (`OWNERS_HOME`), `tools/orient.py:1210`,
      `tools/goals.py:9`, `tools/owners.py:137`. `python tools/check-links.py` is the gate.
- [ ] **Stage 4 and stage 5 after it** — `gh run list` on `main`, then `tools/plan.py:425`'s
      `--past` over all eleven and `--sync` into each `Carried by` cell.
      `docs/agent/loop-goal.md:70` § *Stage 4* says the billing block is its own `BLOCKED`.

## Backlog

- The six keys' reasons are `docs/agent/carried-gaps.md:62-78`; they must move to a module doc or go
  with the file, not into a new index — `docs/agent/loop-goal.md` § *Standing decisions*.
- `tools/owners.py`'s `--untagged-is-an-error`, `--reasons` and `--past-is-an-error` are accepted
  no-ops held open by carried floor checks; they go when the floor stops passing them.
- `docs/agent/carried-gaps.md` still counts as the sixth register in `owners.py --registers`; stage
  3's check wants `5 register(s)` — `docs/agent/loop-goal.toml:11785`.
