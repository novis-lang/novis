# Handoff

## State

Goal `core-arr-1-4` is reached. Its acceptance check is green — `nothing owed` over the 14 members it
names, 42 examples and 14 attacks with 0 failed — and `python tools/dossier.py --owed` names no
`Core\Arr` member at all. All 56 carry `about.md`, three examples, one attack, one bench with a row in
`docs/perf/members.ndjson`, and a test from Novis and from Rust.

The floor check that held the previous DONE claim is fixed: `ThreadSanitizer is clean over nvs-host and
nvs-runtime` failed on `timer::tests::a_timer_and_a_deadline_are_the_same_wheel`, which armed the
socket's deadline before the fixture was built rather than inside the task that waits on it. `bash
tools/tsan.sh` now prints `tsan: clean`. The playbook bullet this session added owns the trap; nothing
in `nvs-host`'s own behaviour moved, only the test's arming point and its two margins.

Nothing is blocked.

## Next group

**Stage 2: the dossier, over the three sibling `Core\Arr` goals** — one file set:
`docs/agent/goals/dossier/91-core-arr-2-4.toml`, `92-core-arr-3-4.toml` and `93-core-arr-4-4.toml`,
plus whatever a red member names under `docs/examples/core/Arr/`, `tests/hostile/core/Arr/` and
`benches/members/core/Arr/`. Every member these three name is already complete, so each should be
green on its first session and the work is to run the check and close the goal;
`rule:testing/feature-proofs` is what each check spells.

- [ ] **Goal `core-arr-2-4` closes on its own check.** Run its `argv` and expect `nothing owed`, `0
      failed`, `0 failed`; the 14 members it names all report `complete` today.
      `docs/agent/goals/dossier/91-core-arr-2-4.toml:76`
- [ ] **Goal `core-arr-3-4` closes the same way**, over the 14 members its own check names.
      `docs/agent/goals/dossier/92-core-arr-3-4.toml:76`
- [ ] **Goal `core-arr-4-4` closes the same way**, and is the last of the class.
      `docs/agent/goals/dossier/93-core-arr-4-4.toml:76`

## Backlog

- A `Core\Arr` member's bench row is re-measured only when its implementing file's text moves
  (`rule:testing/member-perf-ledger`); `crates/nvs-stdlib/src/arr.rs` is untouched this session, so no
  row is stale.
- `docs/agent/loop-goal.toml`'s floor carries two more wall-clock tests that arm a deadline before the
  scheduler starts, `net::tests::a_read_past_its_deadline_reports_a_timeout` and
  `tls::tests::a_handshake_past_its_deadline_reports_a_timeout`. Neither asserts on parking, so neither
  is flaky the same way; leave them alone unless one goes red.
