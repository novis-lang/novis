# Handoff

## State

Goal `core-cache-and-5-more` is complete. All fourteen of its features carry their feature proofs:
the three tier members, the three plain store operations, the sealed half, the two CLDR members, and
now `Core\Cap::has`, `Core\Task\Channel::send` and `Core\Task\Channel::close`. Each carries
`about.md`, three examples with blessed `.out`, one attack, one bench with a ledger row, and a test
from each side. `Core\Cache::shared` stays a recorded `perf` skip in
`tools/data/dossier-policy.toml`. The goal's six `dossier: <group>` checks in stage 2 are what says
it is finished, and all six pass.

`Core\Cap::has` needed four new `[[app]]` blocks in `nvs.toml`: a member that reports grants has
nothing to report on a deployment that granted nothing, so two of its examples, its attack and its
bench each run under one grant. Its Rust proof is a new sweep in
`crates/nvs-stdlib/tests/capability.rs` over the whole `Cap::ALL` roster. Both channel members are
`covers:` markers on unit tests already standing in `crates/nvs-stdlib/src/channel.rs`; the two
`.nvst` cases are new, since the type had none.

Nothing is blocked. `python tools/owners.py --closes core-cache-and-5-more` and `python
tools/playbook.py --closes core-cache-and-5-more` both report that this goal owns nothing, and
`python tools/verify.py --doc` is green.

## Next group

**The next goal's stage 1** — this goal is met, so the chain moves on and this handoff is replaced
wholesale by the next goal's own first session. Until the driver switches, the only work left here is
the goal-end sweep the driver runs itself.

- [ ] Confirm the six stage-2 `dossier: <group>` checks still pass as a set, rather than one group at
      a time as this session ran them. `docs/agent/loop-goal.toml:12622`
- [ ] Re-measure nothing: the three figures this session appended are current for the binary that
      produced them. `docs/perf/members.ndjson:1`

## Backlog

- `Core\Task\Channel::send` costs two allocations and about 99 bytes per value queued; `close` three
  and 288 bytes per channel made and closed. Neither is declared in its bench, because neither is a
  contract — `benches/members/README.md` § the declared counts.
- A channel whose producer never closes it deadlocks with no diagnostic; only the reader's own
  timeout ends it. Nothing owns that today — `crates/nvs-stdlib/src/channel.rs`'s module doc names
  `close()` as the prevention and not a detection.
