# Handoff

## State

Goal `lang-concurrency`. Seven of its ten features now carry every feature proof: the four that
landed before, plus `what-does-not-exist`, `core-task-all-a-fixed-set-of-tasks` and
`core-task-map-one-task-per-element`. Three are still owed — `python tools/dossier.py --owed --group
'lang:concurrency'` is the list.

Nothing is blocked. `lang:concurrency/what-does-not-exist` is `[skip]`ped for perf alone in
`tools/data/dossier-policy.toml:64`, with the reason as its value, the same way
`lang:iteration/what-does-not-exist` is: every name in it is a compile error, so there is no program
to iterate. Everything the chapter's § *What does not exist* claims was checked against the binary
first — `async`, `Fiber`, `Thread`, `Worker`, `parallel\Runtime`, `pcntl_fork`, `curl_multi_init`,
`spawn worker`, an `fn` literal entry, `global`, `$GLOBALS` and a function-scope `static` all refuse,
and a generator has no `send`, `throw` or `getReturn`.

## Next group

The three features the chapter still owes proofs for — one file set:
`docs/reference/lang/80-concurrency.md`, `docs/examples/lang/concurrency/`,
`tests/hostile/lang/concurrency/`, `benches/members/lang/concurrency/` and `tests/conformance/task/`.

- [ ] **`lang:concurrency/core-task-channel-t-a-bounded-queue-between-tasks`** — owes examples,
      hostile, perf, tests (`rule:testing/feature-proofs`). The capacity bound is already pinned by
      `tests/conformance/task/a-bounded-channel-suspends-its-sender.nvst`, so the two new cases are
      the other claims: what `close` does to a waiting receiver, and the type the queue carries.
      `docs/reference/lang/80-concurrency.md:147`
- [ ] **`lang:concurrency/limit-and-deadline`** — owes examples, hostile, perf, tests
      (`rule:concurrency/limit-and-deadline-are-the-only-bounds`). Two landed cases already pin the
      deadline; write the `limit` half, and note that `Core\Time\Duration::milliseconds(int)` is the
      spelling — there is no `fromMillis`. `docs/reference/lang/80-concurrency.md:76`
- [ ] **`lang:concurrency/no-colouring-i-o-just-waits`** — owes examples, hostile, perf, tests
      (`rule:concurrency/one-scheduler`). Its § *What does not exist* half is now proved, so what is
      left is the positive claim: a member that waits parks its own task and the code around it stays
      sequential. `docs/reference/lang/80-concurrency.md:8`

## Backlog

- `docs/reference/findings.md:270`'s D11 still records the old refusal of a static-method spawn entry
  and is stale against the binary — `docs/reference/findings.md`.
- `E0802`'s help text tells the reader an `fn` literal "in `spawn worker` captures its enclosing
  scope", and `spawn worker` is a form the chapter says does not exist — `crates/nvs-diagnostics`.
- Seven `dossier` groups after `lang:concurrency` still owe proofs — `python tools/dossier.py --owed`.
