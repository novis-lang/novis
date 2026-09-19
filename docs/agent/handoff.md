# Handoff

## State

Goal `lang-programs` is finished: all nine features carry the five artefacts, and `python
tools/dossier.py --verify --group lang:programs` reports nothing owed and both suites clean.
`ending-a-program` landed this session — `about.md`, three examples, one attack, `covers:` markers
on the two `exit` cases already in `tests/conformance/lang/`, and a `[skip]` for its perf proof,
because a program ends once and a bench would measure startup rather than `exit`. Both goal-end
gates are clear: `owners.py --closes lang-programs` and `playbook.py --closes lang-programs` name
nothing, and `verify.py --doc` is green.

The proof runner walked subdirectories while the gate counted only the top level, so the helper
files last session put under `parts/` and `vendor/` ran as though they were cases and failed —
nine examples and one attack. `run_scoped` now builds its file list from the feature directories
the gate itself counts, at `tools/dossier.py:3000`, so the two can no longer disagree about what a
case is. That is what took the group's suites from 36 and 12 files to 27 and 9.

`docs/reference/lang/10-programs.md` § *Ending a program* was silent on the status range. The
runtime reports the low byte on every platform — `crates/nvs-cli/src/main.rs:2545` masks with
`& 0xFF`, as PHP does — so `exit(300)` exits 44 and `exit(-1)` exits 255, and the chapter now says
so. The same paragraph no longer claims `die` does not exist; it parses, solely so `E0228` can
point at `exit`, which is what `rule:statements/exit-is-the-only-termination-keyword` says. That
edit moved the chapter's `impl_hash`, so the group's other eight figures were re-measured into
`docs/perf/members.ndjson`.

## Next group

**Stage 2: the dossier, the opening of goal `lang-types`** — one file set:
`docs/reference/lang/20-types.md`, and a fresh directory each under `docs/examples/lang/types/`
and `tests/hostile/lang/types/`. Shapes are `docs/examples/README.md`, `tests/hostile/README.md`
and `rule:testing/four-proofs`. The goal switch installs
`docs/agent/goals/dossier/80-lang-types.handoff.md` over this file, so these three are what it
already names.

- [ ] **`lang:types/array-t`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/20-types.md:151`
- [ ] **`lang:types/callable-classes-object-shapes`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/20-types.md:186`
- [ ] **`lang:types/every-binding-has-a-type`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/20-types.md:9`

## Backlog

- `nvs fmt` rewrites a quoted string with no escape to single quotes, and the identity corpus
  covers `docs/examples/` and `tests/hostile/`, so a new proof file is formatted before it is
  verified — `crates/nvs-fmt`'s module doc owns the rule.
- No harness can assert a program's exit status: a `.nvst` case has only `--EXPECT--`, and the
  `& 0xFF` narrowing lives in the binary's own `main`, out of reach of a `-p nvs-cli` test. The
  hostile case is the only thing guarding it today, through the runner's crash-shaped-status rule.
- `docs/examples/README.md` § *What an example is* says a helper file goes in a subdirectory
  because the sweep counts the top level; now that the runner agrees, it could say so in one
  sentence instead of explaining the mismatch.
