`nvs test --mutate` checks whether your tests notice when a line of your program is wrong.

Novis runs your tests once. Then it changes your program in one place, such as `>=` to `>`. This
changed copy is a mutant. The tests that ran that line run again. If a test fails, the mutant is
`killed`. If every test passes, the mutant `survived`. That means no test checks the result of
that line.

The report lists the survived mutants by method. The mutation score is the percentage of mutants
that made a test fail. It counts only the mutants on lines that a test ran.

`--mutate=Shop\Cart` makes mutants only in that class. `--since main` makes mutants only on the
lines you edited since the git branch `main`. A class or method with `#[NoMutate]` gets no mutants.

**In plain words:** you hide a small mistake in your code and see if your tests find it.

**Good to know:** every test must pass before the mutants run. A survived mutant does not make
the command fail.

**The examples below** show a mutant that survives, the test that kills it, code that no test
calls, and a class with `#[NoMutate]`.
