# Handoff

## State

Goal `core-arr-2-4` is met and so is `core-arr-3-4`: both groups' own checks report nothing owed,
14 features each, every example and attack green. What holds the run is a **floor** check,
`dossier: types:exception`, and it is not `Core\Arr` work. Its two `Core\Db` attacks time out at 60s
for the driver — three sweeps now, at 13:34 and 16:23 — and are green every way this session could
run them: 5 cold runs at 8 workers in 3.4s, through `observe.py`, on the same `target/release/nvs.exe`
the failing sweep used (built 16:20:59, before it). The machine was idle and its checks sequential;
the neighbouring dossier legs took 2–5s. Peak memory is 42 MB and 35 MB, so the unbounded-attack
hazard at `tools/dossier.py:1162` is not this. The cause is a per-program slowdown of roughly twenty
times inside the driver that nothing in this session could reproduce, so what landed instead is the
instrumentation to locate it on the next occurrence. Nothing is blocked.

## Next group

**Stage 2: the dossier, already satisfied except for one floor check** — one file set:
`tools/dossier.py` and `tests/hostile/types/Db-*/`.

- [ ] **Read the next sweep's `types:exception` FAIL line and act on what its new clause names.**
      A timeout now reports how many lines the attack printed and what the last one was, so
      `having printed 1 line(s)` puts the hang inside `Flood::huge`/`Churn::huge`, and
      `2 line(s)` puts it in the deep-rethrow tail; every step too slow prints none of them at 8
      workers. `rule:testing/a-failing-proof-is-fixed-or-recorded` bounds the repair — the attack is
      not softened and the limit is not raised. `tools/dossier.py:1224`
- [ ] **Confirm goal `core-arr-4-4` owes nothing** rather than writing proofs for it, the same way
      the two goals before it were confirmed: its 14 members sit inside the 56 that
      `python tools/dossier.py --verify --group 'Core\Arr'` already passes.
      `rule:testing/feature-proofs` is what it would owe. `crates/nvs-stdlib/src/arr.rs:545`

## Backlog

- The driver's ~20x per-program slowdown is unexplained; `tools/loop.py:1591`'s `capture()` inherits
  its environment whole, which is the one input this session could not compare.
- `tools/dossier.py:1162` still documents an unbounded hostile sweep; measured harmless here (42 MB
  peak), so it is a real gap but not this failure's.
