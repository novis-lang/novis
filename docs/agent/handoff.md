# Handoff

## State

**Goal `bigint` is met, and both reds the DONE claims before this one fell to were the machine
rather than the tree.** `Core\BigInt` with its 24 members, the conformance cases,
`examples/bigint.nvs` and the three named guard tests are on disk, and the stage-5 gates —
`verify.py --doc`, `owners.py --closes bigint`, `playbook.py --closes bigint`, `chain.py --check` —
are green by hand again. The last red, the floor's `nvs-cli (cost margins)`, read warm 31.9 ms
against cold 58.8 ms inside a check whose own 2m32s was 2m31s of compiling; the same commit, idle,
prints 13.2x against the 4x it names.

**That shadow had a cause and `tools/loop.py` now fixes it.** `release_builds` warms *every*
`--release` check rather than the first in file order — the goal names three and only
`nvs-abi-probe` was prebuilt, so `nvs-cli`'s release harness compiled in the foreground right
before the measurement. Why it recompiles every sweep at all is
`crates/nvs-cli/build.rs:99`, which reruns on `.git/index`.

**The driver process predates both fixes**: pid 8444 is one long-lived `python tools/loop.py`
started 2026-09-17, so neither `asked_again` nor `release_builds` is in the process sweeping now.
If a cost guard flickers a third time and the run halts, the repair is to restart the driver so it
loads `tools/loop.py` as it stands. Nothing is blocked.

## Next group

**Goal `gap-zero`, stage 2: the fatal gate, then the index deleted** — one file set:
`tools/owners.py`, `tools/playbook.py`, `tools/brief.py`,
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`. The switch installs that goal's own seed
handoff over this one (`tools/loop.py:3802`); these anchors were re-checked at this commit.

- [ ] **Retire `unowned`** — `tools/owners.py:506`'s `classify`, `tools/owners.py:320`'s `tag_of`,
      and `crates/nvs-stdlib/tests/spec_registry_coverage.rs:505`'s `owner_problem`.
- [ ] **The full gate by default, run by `verify.py`** — `tools/owners.py:727`'s `run_check`.
- [ ] **Delete `docs/agent/carried-gaps.md` and re-point every reader in the same slice** —
      `tools/brief.py:483`'s `OWNERS_HOME`, `tools/owners.py:8` and `tools/orient.py:1210`.

## Backlog

- A driver that re-execs itself when `tools/loop.py` changes under it — unowned, and a hang risk
  worth the user's call before anyone writes it (`docs/agent/commands.md` § the loop).
- `crates/nvs-cli/build.rs:99`'s `.git/index` trigger costs every sweep a 2m30s release relink;
  whether the stamp needs the index at all is that file's own question.
