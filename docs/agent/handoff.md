# Handoff

## State

Goal `lang:errors` is met. All 12 features print `complete.` from `python tools/dossier.py --id`,
and `python tools/dossier.py --verify --group lang:errors` reports nothing owed with 36 examples and
12 attacks green. `python tools/verify.py` is 14 of 14 green (conformance 2208), `verify.py --doc`
resolves every link, and `owners.py --closes lang:errors` and `playbook.py --closes lang:errors`
name nothing.

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
      tests. `docs/reference/lang/80-concurrency.md:193`. A child isolate is a process-shaped thing,
      so check what an example may spawn before writing three of them.
- [ ] **`lang:concurrency/a-child-shares-nothing`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/80-concurrency.md:276`. The attack here is the one that matters: a child
      that tries to reach its parent's state.
- [ ] **`lang:concurrency/a-child-s-throw`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/80-concurrency.md:276`. Read the chapter's own anchor first; the roster
      derives this feature and `a-child-s-failure-is-a-value` from neighbouring sections.

## Backlog

- `rule:errors/stack-depth` estimates "about 65,000 frames" for its 8 MB ceiling; a small recursive
  function reaches ~4,800 (release 4878, debug 4782), so the 128-byte frame behind that number is
  about 12× low. The bound itself holds and stays catchable — only the estimate is off.
- Two roster entries describe one construct: `lang:errors/throw` beside `lang:statements/throw`, and
  `lang:errors/try-catch-finally` beside `lang:statements/try-catch-finally`. Each owes its own
  proofs because `rule:testing/roster-is-derived` reads both reference chapters.
- `docs/decisions/0079.md` § 15 does not say which calls the `calls` counter counts; the playbook
  bullet above records what it measured.
