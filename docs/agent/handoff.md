# Handoff

## State

Goal `lang:errors` is two of its twelve features in. `lang:errors/an-uncaught-throw` and
`lang:errors/assertion-failures` are `complete.` in `python tools/dossier.py --id`; the other ten owe
everything.

Two things landed under them that the rest of the goal will use. **An example may now declare the
status it ends with** — `// dossier: exit 1`, `docs/examples/README.md` § *`// dossier: exit 1`* —
because a feature whose subject is the *ending* has no program that runs to its last line, and
without it `an-uncaught-throw`, `fatal-errors-limits-no-catch-sees` and anything reaching `exit($n)`
could carry no example at all. **An example for a feature that *is* `Core\Test` may call
`Core\Test`**, which the same README's *Nothing is asserted* bullet now says in one sentence.

`lang:errors/an-uncaught-throw` is excused its perf figure in
`tools/data/dossier-policy.toml`: a program reaches that ending once, so there is no loop, and
`lang:errors/throw` carries the throw's own cost.

## Next group

**Three more features of goal `lang:errors`, one slice each** — one file set: the reference chapter
`docs/reference/lang/70-errors.md`, plus the four proof trees under
`docs/examples/lang/errors/`, `tests/hostile/lang/errors/`, `benches/members/lang/errors/` and
`tests/conformance/error/`. `rule:testing/feature-proofs` is what each owes; `python
tools/dossier.py --id '<feature>'` prints the paths.

- [ ] **`lang:errors/capability-denials-are-catchable`** — owes about, examples, hostile, perf,
      tests. `docs/reference/lang/70-errors.md:438`. Its examples need grants: an example runs from
      the repository root, so a capability one uses is an `[[app]]` block in the root `nvs.toml`
      keyed by that example's own `entry`, the way `examples/capability.nvs` already has one.
- [ ] **`lang:errors/fatal-errors-limits-no-catch-sees`** — owes about, examples, hostile, perf,
      tests. `docs/reference/lang/70-errors.md:396`. Its examples end at a limit, so they carry
      `// dossier: exit 1`; `Core\Fatal::onLimit` is the one hook that observes a `FATAL`.
- [ ] **`lang:errors/constructing-and-subclassing`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:137`. Nothing here ends a program, so its three examples are
      ordinary ones.

## Backlog

- `docs/reference/lang/70-errors.md:361` shows standard error carrying `Uncaught Exception: …` and
  `#0 Class::method() at file:line`, which is `rule:errors/renderings`'s **plaintext** rendering. The
  default is `[log] format = "json"`, so what the binary writes is one JSON Lines record whose
  `nodes` array carries the frames. The block is `text` and nothing runs it. A doc slice.
- The same section says "There is no `set_exception_handler` and no `set_error_handler`" and names
  neither `Core\Fatal::onUncaughtThrow` nor `Core\Script::onExit`, which both run at that ending and
  change nothing about it (`rule:errors/on-uncaught-throw`). The sentence is true and the section is
  thin. A doc slice, in the same pass as the one above.
- `spawn script`'s path is resolved against the working directory
  (`docs/reference/lang/80-concurrency.md:207`), so an example with a companion script cannot name it
  relatively. No example tree has a companion subdirectory yet; the first one to need one pays this.
