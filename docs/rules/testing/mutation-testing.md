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
alone. A condition the checker narrows a type on (an `is` test, or an `==` against a literal, `null`
or a class constant) keeps its direction, its connectors and that test. A `true` or `false` that is a
condition, a `match` subject or an `==` operand keeps its value. A mutant that took a narrowed edge
with the test false would read a value at a type it does not have.

The set is enumerated where it is implemented, so adding an operator changes no rule.

`--mutate` runs on a program's `#[Test]` methods only. On a `.nvst` tree it exits with an error, as
the coverage flags do, because per-test coverage exists only for program tests.

**What it costs:** nothing for a run without `--mutate`, and nothing on the request path. A mutation
run pays one coverage run, then one lowering and codegen of the whole program per mutant, plus the
tests that reach the mutant's line. Memory is O(`--jobs`): the mutants in flight.

**Declined:**

- *Mutating the syntax tree.* Each mutant would pay a check to find out whether it compiles, and the
  tree has no types for a swap to keep.
- *Recompiling only the mutated function.* It needs a partial-recompile mechanism kept correct beside
  the whole-program one, for a cost the per-mutant test run already outweighs.

**Not on disk.** `nvs_ir::lower::mutants` lists the mutants and `lower_mutant` lowers one, but
nothing runs a mutant: `nvs test` has no `--mutate` flag.
