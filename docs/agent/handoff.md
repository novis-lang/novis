# Handoff

## State

**Milestone M8, goal `queue-purge`. Stage 3 is landed:** `Core\Queue::purge(string $queue, {state?,
tag?, before?, limit?}): uint` is written, gated on `queue.purge`, and held by four `-p nvs-stdlib`
tests and three `.nvst` cases. `delete` and the `queue.purge` roster entry were already on disk.
`python tools/verify.py` is green.

**The bound with nothing written is `DEFAULT_PURGE_LIMIT` = 1000**, decided this session because ADR
0153 § 4 names none; the reasoning is that constant's own doc comment in
`crates/nvs-stdlib/src/queue.rs`. The queue name is `Qual::Neutral` by
`rule:security/sink-predicate` — a bound parameter the protocol frames, with the grant, not a
qualifier, as what keeps a request from choosing which queue is swept.

**Open:** stage 4's static half, and stage 6's fixture. `examples/queue-purge.nvs` — the driver's
failing acceptance check — is stage 6 and wants a live database, so it is not the next thing.

**Still a goal bug, not a gap:** stage 4's `-p nvs-types` check names `E0635`, which is already
`E_NO_UNIX_TRANSPORT` at `crates/nvs-diagnostics/src/lib.rs:1778`. The band's next free code is
`E0637`, and the four check names in `docs/agent/loop-goal.toml` have to be re-spelled with whatever
number is chosen before their tests can be written.

## Next group

**Stage 4: the static half, the one diagnostic** — one file set:
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-types/src/intrinsics.rs`,
`crates/nvs-types/tests/intrinsics.rs`, `docs/agent/loop-goal.toml`.

- [ ] **Claim the code and repair the check.** Declare it beside `E_UNGRANTED_HOST` at
      `crates/nvs-diagnostics/src/lib.rs:1510` — the band's next free is `E0637`, re-read at the
      moment of writing — and re-spell the four test names in the `stage = "4 the diagnostic"`
      block at `docs/agent/loop-goal.toml:7316`, which currently say `e0635`. ADR 0153 § 5 and
      `rule:security/capability-check-at-the-door` are what the code is about; it is `E0618` one
      class over and is asked under `E0618`'s conditions and no others.
- [ ] **The check, in `E0618`'s own shape**, at `crates/nvs-types/src/intrinsics.rs:635`: a
      *written* `Core\Queue::purge` queue name that no `[capabilities.queue] purge` entry grants is
      refused while checking, and a computed one is not refused before it runs. `delete` has no
      static half at all — its queue arrives inside a `Queue\Id`.
      `rule:expressions/intrinsic-list-is-closed` is the hook's own rule.
- [ ] **The four `-p nvs-types` tests**, beside `crates/nvs-types/tests/intrinsics.rs:695`, which
      is `reported(&ungranted, code::E_UNGRANTED_HOST)` — the same fixture one grant over. The
      fourth of them asserts `delete` is refused at the door and not while checking.

## Backlog

- `examples/queue-purge.nvs` and its eight frozen `want` lines — stage 6, `docs/agent/loop-goal.toml:7344`.
- The stage 6 conformance and differential suites, and `python tools/reference.py --check`.
- `purge`'s server-side case already exists as `purge_removes_what_has_finished_within_its_filters_and_stops_at_its_limit` in `crates/nvs-stdlib/tests/queue.rs` — container-gated, so it is not run by `verify.py`.
- Carried gaps that outlive this goal are in `docs/agent/carried-gaps.md`.
