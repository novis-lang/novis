# Handoff

## State

Goal `lang:errors` is eight of its twelve features in. `an-uncaught-throw`, `assertion-failures`,
`capability-denials-are-catchable`, `constructing-and-subclassing`,
`fatal-errors-limits-no-catch-sees`, `properties-not-accessors`, `rethrowing` and `the-throwable-tree`
are `complete.` in `python tools/dossier.py --id`; the other four owe everything.

Four things the rest of the goal will use. **The two `.nvst` cases a feature owes are usually already
in `tests/conformance/error/`** — attributing one is a one-line `// covers:` edit in its `--FILE--`
block, the marker takes a comma-separated list, so one case may serve two features. **A marker shifts
the lines under it**, and the playbook bullet above says what that breaks. **An example may declare
the status it ends with** — `// dossier: exit 1`, `docs/examples/README.md` § *`// dossier: exit 1`*.
**An example that needs a capability or a limit is an `[[app]]` block in the root `nvs.toml` keyed by
that example's own `entry`**; a `.nvst` case needs none, since it carries its own `--FILE nvs.toml--`
section.

Two features are excused their perf figure in `tools/data/dossier-policy.toml` because their subject
is an ending: `an-uncaught-throw` and `fatal-errors-limits-no-catch-sees`.

## Next group

**Stage 2: the last four features of goal `lang:errors`, one slice each** — one file set: the
reference chapter `docs/reference/lang/70-errors.md`, plus the four proof trees under
`docs/examples/lang/errors/`, `tests/hostile/lang/errors/`, `benches/members/lang/errors/` and
`tests/conformance/error/`. `rule:testing/feature-proofs` is what each owes; `python
tools/dossier.py --id '<feature>'` prints the paths.

- [ ] **`lang:errors/throw`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:177`. `throw` is a statement and an expression, so the
      examples want it after `??`, in a `match` arm and in a ternary.
- [ ] **`lang:errors/try-catch-finally`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:201`. Clause order, one class per clause, one binding per
      clause, and what `finally` runs on every way out.
- [ ] **`lang:errors/recursion-depth`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:370`. The limit fires at about 3,676 frames on this host;
      measure it again rather than copying that number.
- [ ] **`lang:errors/inspecting-a-value`** — owes about, examples, hostile, perf, tests.
      `docs/reference/lang/70-errors.md:510`.

## Backlog

- A throw and a supertype catch cost 17 allocations and 980 bytes per round, and a rethrow round trip
  22 and 1,237 — `docs/perf/members.ndjson`, features `lang:errors/the-throwable-tree` and
  `lang:errors/rethrowing`. Nothing is wrong with them; they are the first figures anyone has for the
  throw path, and whoever owns `crates/nvs-runtime/src/throwable.rs` may want a look.
- `tests/hostile/lang/errors/properties-not-accessors` reads 3,673 frames out of a chain asked to go
  10,000 deep: the recursion limit ends it first, which goal item `recursion-depth` is about.
