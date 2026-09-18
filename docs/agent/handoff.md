# Handoff

## State

Goal `config-directives-3-3` is **met**: all fourteen `nvs.toml` directives it names own their four
artefacts, and `python tools/dossier.py --verify --only <the fourteen>` reports `nothing owed`,
`0 failed`, `0 failed`. This session closed the last seven — `[queue]`'s four keys, `schedule`,
`server` and `session`. The three end-of-goal gates are clean: `verify.py --doc` is green,
`owners.py --closes` and `playbook.py --closes` each name nothing.

A directive owes four artefacts and not five — `about.md`, one example with its blessed `.out`, one
attack, one `covers:`-marked case — because the policy excuses a directive from the perf proof.

Every test this session needed already existed; the edit was the `covers:` marker, plus one row
added to `every_queue_that_cannot_run_a_job_is_refused` for the empty-connection spelling
`connection_of` filters (`crates/nvs-config/src/queue.rs:160`). The playbook bullet above is that
trap.

`[context] modules` named the registry, `tree.rs`, `export.rs`, `cache.rs` and `tests/directives.rs`,
and the work needed five files none of them covers: `src/queue.rs`, `src/schedule.rs`,
`src/session.rs` hold the boot pass a directive's example and attack are written against, and
`tests/queue.rs`, `tests/resolve.rs`, `tests/snapshot.rs` hold the cases that were owed a marker. The
gap is the generator's, not this goal's — `tools/dossier.py:2799` seeds `modules` from the
implementing anchor alone, so every generated `config:*` goal has it.

## Next group

**Goal met — the chain moves to `types-enum-1-2`, which installs its own handoff on the switch.**
Nothing in this goal's file set is open.

- [x] `directive:queue.connection` — page, example, attack; the two registry cases marked
- [x] `directive:queue.workers` — page, example, attack
- [x] `directive:queue.max_attempts` — page, example, attack
- [x] `directive:queue.visibility` — page, example, attack
- [x] `directive:schedule` — page, example, attack; `resolve.rs`'s spawn-roots case marked
- [x] `directive:server` — page, example, attack; `snapshot.rs`'s whole-block case marked
- [x] `directive:session` — page, example, attack; `session.rs`'s backend-roster case marked

## Backlog

- A generated `config:*` goal's `[context] modules` misses the crate's boot-pass module and the test
  files that already hold the claims — `tools/dossier.py:2799`.
- 1086 features still owe a proof tree-wide (`python tools/dossier.py --owed`); the dossier goals
  after this one are the schedule for them.
