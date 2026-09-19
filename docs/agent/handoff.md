# Handoff

## State

Goal `lang:errors` is met. All 12 features print `complete.` from `python tools/dossier.py --id`,
and `python tools/dossier.py --verify --group lang:errors` reports nothing owed with 36 examples and
12 attacks green. `python tools/verify.py` is 14 of 14 green (conformance 2208, differential 279,
4877 tests), `verify.py --doc` resolves every link, and `owners.py --closes lang:errors` and
`playbook.py --closes lang:errors` name nothing.

The previous session's DONE claim failed its sweep on one floor check — `the CLI's own start work
stays under 6ms`. That was the measurement's input rather than the tree: `--warm-start` ran `nvs
run` with no `--config`, so it resolved this repository's own `nvs.toml`, whose `[[app]]` array has
gone from twelve blocks to thirty-two since the budget was written. The leg now names
`crates/nvs-config/src/default.toml` and reads 4.2 ms against the 6.0 ms budget, which is the margin
the budget was authored with. `tools/bench.py`'s `warm_start` docstring owns the decomposition.

Two features stay excused their perf figure in `tools/data/dossier-policy.toml` because their
subject is an ending: `an-uncaught-throw` and `fatal-errors-limits-no-catch-sees`.

## Next group

**Stage 2: the first features of goal `lang:concurrency`, one slice each** — one file set: the
reference chapter `docs/reference/lang/80-concurrency.md`, plus the proof trees under
`docs/examples/lang/concurrency/`, `tests/hostile/lang/concurrency/`,
`benches/members/lang/concurrency/` and `tests/conformance/`. `rule:testing/feature-proofs` is what
each owes, and `python tools/dossier.py --id '<feature>'` prints the path of each artefact. All 10
features of that goal owe everything.

- [ ] **`lang:concurrency/isolates-spawn-script-and-await`** — owes about, examples, hostile, perf,
      tests. `docs/reference/lang/80-concurrency.md:192`. A child isolate is a process-shaped thing,
      so check what an example may spawn before writing three of them.
- [ ] **`lang:concurrency/a-child-shares-nothing`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/80-concurrency.md:275`.
- [ ] **`lang:concurrency/a-child-s-throw`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/80-concurrency.md:119`.

## Backlog

- Config resolution is priced per live directive, not per byte: 30 KB of comments cost 0.04 ms where
  207 directives cost 1.8 ms, and an `[[app]]` block is about 42 µs. Nothing owns this figure yet;
  `docs/perf/` has no startup note. (`tools/bench.py`'s `warm_start`)
- `bench.py`'s suite and `--serve-vs-fpm` legs still resolve `./nvs.toml` implicitly. Their figures
  are tens of ms, so 1.9 ms is noise there, but the input is unnamed the same way. (`tools/bench.py`)
