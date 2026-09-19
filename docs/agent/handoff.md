# Handoff

## State

Goal `types-interface` is met: `python tools/dossier.py --verify --group 'types:interface'` prints
`nothing owed`, and both DONE gates own nothing (`owners.py --closes types-interface`,
`playbook.py --closes types-interface`). What stopped the goal switch was the floor's
`nvs-cli (cost margins)` check, and that is the machine rather than the tree — the only Rust the
goal touched was two doc comments, the guard's cold arm was unchanged at 57ms through both red
asks, and the same binary at the same commit reads 12.3x against its 4x margin once the box is
idle. `tools/loop.py`'s `COST_SETTLES` now gives a red cost guard the recovery time that was
measured, so the next full sweep waits the shadow out instead of reporting it.

## Next group

**Goal `lang-programs`, stage 2 — one file set: `docs/reference/lang/10-programs.md`.** The chain
advances at the goal switch and installs that goal's own sibling handoff; these are its first three
items, and each owes all four proofs rather than the two an interface owes:

- [ ] **`lang:programs/a-program-is-a-file-of-top-level-statements`** — owes examples, hostile, perf,
      tests. `docs/reference/lang/10-programs.md:8`
- [ ] **`lang:programs/a-complete-program-annotated`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/10-programs.md:25`
- [ ] **`lang:programs/also-a-line-comment`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/10-programs.md:113`

## Backlog

- The release units behind `cargo test --release -p nvs-cli --bin nvs` were invalidated between two
  identical invocations twenty minutes apart — a 2m10s rebuild with nothing edited — and a
  `git status` on its own did not reproduce it, so the `.git/index` attribution in
  `docs/agent/playbook.md`'s bullet is unconfirmed and every sweep that runs the release checks
  still pays that link.
- The warm-start margin has no control of its own, so it cannot tell a slow loader from a dilated
  machine from the inside; `benches/abi-probe/tests/perf_guards.rs`'s fan-out guard is the shape
  that answers that, and `tools/bench.py`'s `QUIET_FLOOR_MS` is the other one.
- `Parses` seeds a `tryParse` default with no compiled function, so an implementor cannot reach it;
  the open question is whether that body gets compiled or the seed goes, since
  `rule:expressions/try-parse` gives a laundering `parse` no non-throwing twin at all. Analysis in
  `.loop/dossier-findings/w03-parses.md`, mechanism in `docs/agent/playbook.md:7488`.
- Three `Stringable` shapes still unasserted: a child that inherits `toString` without overriding it
  rendered at an implicit site, `Stringable` as a return type, and handing a `Stringable` to a
  parameter typed `string` — the last belongs under `tests/conformance/reject/`.
- The fan-out is the cheapest way to walk a generated dossier goal: five workers landed five
  features in one session's context, and the parent's window went to reviewing rather than writing.
