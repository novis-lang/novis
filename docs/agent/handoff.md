# Handoff

## State

**Goal `dossier` is met.** `python tools/dossier.py --emit-goals` wrote **100 goals over 1,132 owed
features** into `docs/agent/goals/dossier/`, appended to the end of the chain with `ci-green`
renumbered behind them; the chain is 172 entries and `chain.py --check`, `plan.py --check` and
`dossier.py --check-goals` are all green, as is the `--dry-run` that says the chain already names
every one of them. Stage 3's report is `.loop/optimization/report.md` (gitignored, so it is the only
copy).

The one red check a DONE claim fell to was this file: its State named the generated goals by their
numbers, which `chain.py --check` refuses tree-wide and no wrap gate saw. `session.py`'s
`body_goal_numbers` now refuses that shape in any body a wrap writes, commit messages included, so
the next one cannot be authored — a number is a position, and only `29 of 43` beside a total is one.

Stage 3's other find is landed: `calibrate()` clamped a failed calibration to one picosecond instead
of refusing it, and appended a `units` figure of 1.1e11 to the append-only ledger. The per-feature
estimate the fan-out rests on was priced by hand, as the goal's § *Standing decisions* permits:
**~20 calls, against the estimated 16**. That scales both legs alike, so `FANOUT_WORKERS` stays at 8
and `--per-goal` at 18; it moves the schedule, not the constants.

## Next group

**Stage 3: the loop is ready** — one file set, `tools/dossier.py` and `docs/agent/goals/dossier/`.
Nothing here is open; these are the items a retry would take if the DONE sweep comes back red.

- [ ] **If a generated goal's check is red, the fix is in the emitter, never the file** —
      `tools/dossier.py:2282` (`goal_toml`) and `tools/dossier.py:2278` (`goal_prose`), then
      re-emit. A hand-edit under `docs/agent/goals/dossier/` is lost at the next emission and the
      header of every generated file says so. `rule:testing/roster-is-derived`.
- [ ] **If `--record-perf` refuses with "the calibration did not measure anything", the machine is
      busy and that is the guard working** — `tools/dossier.py:1244`, bound at
      `tools/dossier.py:333`. Re-run it idle; on this machine an idle unit reads 3.3 ns/iteration.
      `rule:testing/member-perf-ledger`.
- [ ] **A wrap refusal naming a goal number is this gate, and it takes the slug** —
      `tools/session.py:1143` (`body_goal_numbers`). It reuses `chain.py`'s regex and its three
      exempt headers, so what it refuses is exactly what the floor refuses.

## Backlog
- The generated pack is 38,168 B against this goal's 25,182 B; the traps section is 4,515 B of
  lead-ins nobody promotes — `.loop/optimization/report.md` § *Proposals* 1.
- 181 floor checks are 181 process launches at 1.48s each; `--verify` taking several `--group`
  values would make it 100 — same file, *Proposals* 2.
- The four-proofs shape does not warn that a conversion is `expr as T` and nothing else; three of
  my ~20 calls were that — `docs/agent/conventions.md` § *A feature's four proofs*, *Proposals* 3.
- `.loop/logs/` holds 4 sessions, so the 68-session base of `FANOUT_WORKERS`' derivation and the
  28% `writing` share are no longer re-derivable — same file, *Blocked*.
