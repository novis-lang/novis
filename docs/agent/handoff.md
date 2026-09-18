# Handoff

## State

**Goal `gap-zero` is met.** The DONE claim that failed its sweep failed on a flaky floor check, not on
the tree: `conformance (Core by name)` read `2082 passed, 1 failed` because
`tests/conformance/core/process-a-capture-is-whole-not-a-pipes-worth.nvst` was killed at the runner's
deadline. The case's answer was never wrong — the suite passes 2083 of 2083 at the commit the sweep
failed on.

**`Core\Process::run` is not implicated.** `crates/nvs-stdlib/src/process.rs:707`'s `drain` reads both
pipes on concurrent readers and only then waits, so the deadlock the case exists to catch is not what
it hit. What the case costs is eight OS process spawns and the draining of what they write, and both
are set by how contended the machine is rather than by what the case computes.

**Two things landed.** The case's Windows child now writes whole thousand-octet lines plus the
remainder instead of one line per hundred octets — every size, every count and every `--EXPECT--` line
is unchanged, and the case is faster on an idle machine than before. And
`nvs_test::run::CASE_TIMEOUT` is 300s: its own doc claimed headroom over "a case that is merely slow"
that it did not have for a case whose cost is process creation.

Measured on this machine under `nproc`-many busy loops, one leg of the case took 177s when its child
emitted 262,100 octets per stream as 2,621 line-sized writes and 4.2s when the same bytes arrived as
262 larger ones, with the child itself costing the same either way. The full case stayed noisy across
that change because its eight spawns dominate and spawn cost under saturation swings several-fold,
which is the variance the deadline now absorbs rather than reports.

## Next group

**Goal `dossier` — its own seed replaces this file at the switch** — one file set:
`docs/agent/goals/71-dossier.*`.

- [ ] **Take goal `dossier`'s first item from its seed handoff** —
      `docs/agent/goals/71-dossier.handoff.md:1` names the group and the file set it shares; the
      driver installs it when it walks past goal `gap-zero`, so nothing here needs carrying forward.

## Backlog

- The deadline now clears the observed overrun with room, but nothing proves the flake gone; if the
  case reddens again the next lever is the driver not pooling the conformance suite beside cargo
  builds — `tools/loop.py`.
