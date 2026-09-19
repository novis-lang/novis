# Handoff

## State

Goal `lang-concurrency`. Four of its ten features now carry every feature proof:
`a-child-s-failure-is-a-value`, `a-child-s-throw`, `a-child-shares-nothing` and
`isolates-spawn-script-and-await`. Six are still owed — `python tools/dossier.py --owed --group
'lang:concurrency'` is the list.

Nothing is blocked. Writing the spawn proofs turned up three stale sentences in
`docs/reference/lang/80-concurrency.md`, all fixed this session against the binary and the landed
conformance cases: a child with no top-level `return` answers `null` (ADR 0006 § *Decision* said so
all along), `args:` is read by `Core\Script::args()`, and a spawn entry may be a static method
written `Class::method(...)` with `limits:`, `grants:` and `on:` all accepted — only an `fn` literal
is refused (`E0802`). `docs/reference/findings.md:270`'s D11 still records the old refusal and is
now stale.

## Next group

The `Core\Task` half of the chapter, and the section that tells the reader what is not there — one
file set: `docs/reference/lang/80-concurrency.md`, `docs/examples/lang/concurrency/`,
`tests/hostile/lang/concurrency/`, `benches/members/lang/concurrency/`, `tests/conformance/task/`
and `tools/data/dossier-policy.toml`.

- [ ] **`lang:concurrency/what-does-not-exist`** — owes examples, hostile, perf, tests
      (`rule:testing/feature-proofs`). Its list changed this session, so write the proofs from the
      section as it now reads. `lang:iteration/what-does-not-exist` is the precedent for all of it:
      `tools/data/dossier-policy.toml:67` skips **perf only**, with the reason as its value, and its
      three examples show the replacement rather than the missing name, while its attack gives the
      compiler every dropped name at once under `// hostile: expect-refusal`.
      `docs/reference/lang/80-concurrency.md:382`
- [ ] **`lang:concurrency/core-task-all-a-fixed-set-of-tasks`** — owes examples, hostile, perf, tests
      (`rule:testing/feature-proofs`). The shape-of-callables subject and the refusal of a subject
      that is not one are both in the section. `docs/reference/lang/80-concurrency.md:20`
- [ ] **`lang:concurrency/core-task-map-one-task-per-element`** — owes examples, hostile, perf, tests
      (`rule:testing/feature-proofs`). Results come back under the input's keys and in the input's
      order, which is the invariant an attack should try to break.
      `docs/reference/lang/80-concurrency.md:49`

## Backlog

- `no-colouring-i-o-just-waits`, `limit-and-deadline` and `core-task-channel-t-a-bounded-queue-between-tasks`
  still owe every proof — `python tools/dossier.py --owed --group 'lang:concurrency'`.
- `docs/reference/findings.md:270`'s D11 is unticked and says `spawn script Class::method(...)` is
  refused; the tree runs one, so the row wants ticking with what landed.
- `Core\Script::args` owes examples, hostile, perf and a Rust-side `covers:` marker — its own group.
