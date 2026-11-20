`nvs test --mutate` checks whether your tests notice when a line of your program is wrong.

Novis runs your tests once. Then it changes your program in one place, such as `>=` to `>`. This
changed copy is a mutant. The tests that run that line run again. If a test fails, the mutant is
`killed`. If every test passes, it `survived`, and no test checks that line's result.

The report lists the survived mutants by method. The mutation score is the percentage of reached
mutants that made a test fail.

`--mutate=Shop\Cart` changes only that class. `--since main` changes only the lines you edited
since `main`. A class or method with `#[NoMutate]` is never changed.

**In plain words:** you hide a small mistake in your code and see if your tests find it.

**Good to know:** every test must pass before the mutants run. A survived mutant does not make
the command fail.

**The examples below** show a mutant that survives, the test that kills it, code that no test
calls, and a class with `#[NoMutate]`.
