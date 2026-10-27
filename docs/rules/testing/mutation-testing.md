A mutation run makes a small, deliberate change to the code, re-runs the tests, and records whether
anything failed. It is the measurement that closes the gap coverage leaves: a line having run says
nothing about whether an assertion would have objected had it been wrong, and a suite at full line
coverage routinely misses most introduced defects.

Two things make the built-in version categorically better than an external one, and neither is
available outside the compiler. Mutants are generated from typed IR, so every one is type-valid and
compiles, where a tool mutating a syntax tree pays a compile to discover that many do not. And
mutant runs are **coverage-directed**: the probe data says which tests touch the mutated line, so a
mutant runs against those rather than against the whole suite. On two thousand tests and five
hundred mutants that is the difference between minutes and most of a day.

**A mutant is one changed site in the lowered program.** Parse and check run once; lowering takes the
checked tree and a mutant key, makes exactly one change at the site the key names, and codegen
compiles the whole program from that. A key is the site's file, line and change, so a report names a
mutant the way a reader finds it. No mutant is made from test code
(`rule:testing/tests-never-reach-a-build`'s marker methods, and every body written inside one), so a
mutant changes the program under test and never the test that judges it.

The first operator set is six swaps, each keeping its operands' and its result's types, so every
mutant lowers and compiles with no second check:

| Operator | Change |
|---|---|
| relational boundary | `<` ↔ `<=`, `>` ↔ `>=` |
| equality negated | `==` ↔ `!=` |
| arithmetic swap | `+` ↔ `-`, `*` ↔ `/` |
| logical connector | `&&` ↔ `\|\|` |
| boolean literal | `true` ↔ `false` |
| branch condition negated | an `if`'s or a loop's condition is inverted |

A swap is made only where it keeps the result type, so `*` and `/` swap over `float` and `decimal`
alone. A condition whose narrowing changes how a value is read keeps its direction, its connectors
and that test: an `is` test, and an `==` or `!=` that compares a value stored tagged (`?T`, a union,
`mixed`) with a literal, `null` or a class constant. A mutant that took such an edge with the test
false would read a value at a representation it does not have. An equality over a value stored any
other way narrows it to a single value or an enum case, which is stored as its base, so
`int $n == 10` and `$mode == Mode::Off` are reversed and their branches negated. A variable is judged
by its slot, which an earlier test does not untag, so the line is drawn on the safe side. A `true` or
`false` that is a condition, a `match` subject or an `==` operand keeps its value.

The set is enumerated where it is implemented, so adding an operator changes no rule.

`--mutate` runs on a program's `#[Test]` methods only. On a `.nvst` tree it exits with an error, as
the coverage flags do, because per-test coverage exists only for program tests.

**The run.** `nvs test --mutate` runs the selected tests once with coverage, as a coverage flag does
(`rule:testing/coverage-report`), and reports that run as any run is reported. A test that fails there
fails the run, and no mutant runs: a mutant is judged against a suite that passes without it. Then
each mutant runs the selected tests that reached the statement it changes, the innermost statement
whose span holds the changed expression, up to `--jobs` at once, and no further test starts once one
has failed. A test with no wall-time limit gets one for a mutant run, ten times the slowest test of
the coverage run plus one second, because a mutant can make a loop that never ends; a test with its
own limit keeps it (`rule:testing/a-test-has-a-wall-time-limit`).

Each mutant has one of four outcomes:

| Outcome | When |
|---|---|
| `killed` | a test failed or called `exit` |
| `timed out` | no test failed, and a test ran past its wall-time limit |
| `survived` | every test it ran passed, a flaky one included |
| `not reached` | no selected test reached its statement, so nothing ran |

The score is killed plus timed out over every mutant but the unreached ones. The human report prints
the four counts and the score after the run's summary, then each survived mutant with its file, line
and change. `--format json` writes `schemaVersion: 4` (`rule:testing/report-formats`): version 3's
document, plus a `mutants` array with one record per mutant, `{file, line, change, outcome, tests}`,
in the order the program lowers its sites. `tests` is how many tests reached the mutant's line, which
is what the covering-test filter decides and what the bench of a mutation run counts. A JUnit document has no place for a mutant, so `--mutate` with
`--format junit` exits with an error. The exit status is the coverage run's: a survived mutant does not
fail the run.

**What it costs:** nothing for a run without `--mutate`, and nothing on the request path. A mutation
run pays one coverage run, then one lowering and codegen of the whole program per mutant, plus the
tests that reach the mutant's line. A mutant compiles with no artifact cache, since its key would be
the unchanged program's. Memory is O(`--jobs`): one mutant runs at a time, with its tests in flight,
and the run keeps one record per mutant for the report.

Since the mutants grow with the program and each compiles all of it, a run's compile time grows with
the square of the program's size; that is the price of the whole-program recompile, and a team runs
`--mutate` on a schedule, such as a nightly CI job, not on every save. What the covering-test
filter decides is measured: `benches/scaling/testing/mutate.nvs` grows the suite around a fixed
program under test, and its ramp fails when the test runs per mutant grow with the suite.

**Declined:**

- *Keeping every equality against a literal, `null` or a constant*, which the syntax alone decides.
  Nearly every condition an application writes is one, so the equality and branch operators made no
  mutant in most programs.
- *Mutating the syntax tree.* Each mutant would pay a check to find out whether it compiles, and the
  tree has no types for a swap to keep.
- *Recompiling only the mutated function.* It needs a partial-recompile mechanism kept correct beside
  the whole-program one, for a cost the per-mutant test run already outweighs.
- *Running several mutants at once.* Each would hold a compiled unit and its tests' isolates, and
  `--jobs` already fills the cores with one mutant's tests.
- *A per-test limit from each test's own coverage-run time.* It is closer for a fast test, and it
  needs a table carried into every test's run; one bound for the run is one number.
