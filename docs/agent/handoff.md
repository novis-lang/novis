# Handoff

## State

**Goal `worker-placement` is met: the one red floor check is green and every stage claim stands.**
The failure was a title, not owed work. `crates/nvs-runtime/src/script.rs:90` now reads
*Reaching a core that starts later*: the section describes the published `SharedResolver` handle a
core installs as it starts, which is landed, while `owners.py`'s `OWED` pattern matched `not … yet`
in the old heading and counted it as owed work written where no owner tag reaches it. The three
citations of that title moved with it — `crates/nvs-cli/src/main.rs:2400`,
`crates/nvs-cli/src/serve.rs:420`, `crates/nvs-runtime/src/script.rs:302`.

`python tools/owners.py` reports `sections outside Known gaps: 0`, `owners.py --closes
worker-placement` and `playbook.py --closes worker-placement` each report the goal owns nothing, and
`verify.py --doc` is green. Nothing is blocked.

## Next group

**Goal switch: `core-class-tests` opens with its own handoff** — one file set:
`docs/agent/goals/63-core-class-tests.*`.

- [ ] **Take the next goal's first item from its own handoff** — the driver installs
      `docs/agent/goals/63-core-class-tests.handoff.md:1` over this file at the switch, so nothing of
      `worker-placement` carries into it except what `docs/agent/carried-gaps.md` still holds.

## Backlog

- `crates/nvs-runtime/src/graph.rs:74` gap 1 — a decoded `Core` instance is a `mixed` a program
  cannot narrow — is goal `core-class-tests`' own, per `python tools/owners.py`.
