# Handoff

## State

Goal `lang:errors` is four of its twelve features in. `lang:errors/an-uncaught-throw`,
`assertion-failures`, `capability-denials-are-catchable` and `fatal-errors-limits-no-catch-sees` are
`complete.` in `python tools/dossier.py --id`; the other eight owe everything.

Three things the rest of the goal will use. **An example may declare the status it ends with** —
`// dossier: exit 1`, `docs/examples/README.md` § *`// dossier: exit 1`* — which is what a feature
whose subject is the *ending* carries instead of a program that runs to its last line. **An example
that needs a capability or a limit is an `[[app]]` block in the root `nvs.toml` keyed by that
example's own `entry`**, because an example runs from the repository root; `[app.limits] memory =
"16M"` is the ceiling the conformance cases are already written around, and two 8 MiB slabs breach
it. **A `.nvst` case needs no such block**: it carries its own `--FILE nvs.toml--` section.

Two features are excused their perf figure in `tools/data/dossier-policy.toml` for one reason —
their subject is an ending, so there is no loop: `an-uncaught-throw` and
`fatal-errors-limits-no-catch-sees`.

## Next group

**Three more features of goal `lang:errors`, one slice each** — one file set: the reference chapter
`docs/reference/lang/70-errors.md`, plus the four proof trees under
`docs/examples/lang/errors/`, `tests/hostile/lang/errors/`, `benches/members/lang/errors/` and
`tests/conformance/error/`. `rule:testing/feature-proofs` is what each owes; `python
tools/dossier.py --id '<feature>'` prints the paths.

- [ ] **`lang:errors/constructing-and-subclassing`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:136`. A user class extends `Throwable` directly, so the
      examples are a program's own exception class and the `{previous: …}` chain.
- [ ] **`lang:errors/the-throwable-tree`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:8`. `tests/conformance/error/` already holds cases that pin
      the tree; check what a `covers:` marker alone would attribute before writing a new one.
- [ ] **`lang:errors/properties-not-accessors`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:79`. The property table is at
      `docs/reference/lang/70-errors.md:84`; pin `backtrace` rather than `location` for anything a
      `Core` member raised, for the reason the playbook's new bullet gives.

## Backlog

- Five features of this goal after the group above: `throw` (`70-errors.md:176`), `try-catch-finally`
  (`:200`), `rethrowing` (`:309`), `recursion-depth` (`:369`), `inspecting-a-value` (`:509`).
- `docs/reference/lang/70-errors.md:88` says `location` is the `file:line` of the `throw` and empty
  only until the object is thrown, which a fault raised inside a `Core` member disagrees with — it
  reads empty unless the frame that raised it also catches it. The tested behaviour is the one to
  keep; the sentence is what needs the qualifier.
- A denied `Core\IO::read` costs 846 ns, 21 allocations and 1754 bytes for a call that does no work
  (`docs/perf/members.ndjson`) — the message and its `help:` line are built before anyone asks for
  them.
