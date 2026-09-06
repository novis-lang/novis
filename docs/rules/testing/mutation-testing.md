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

The operator set is enumerated where it is implemented, so adding an operator changes no rule.
