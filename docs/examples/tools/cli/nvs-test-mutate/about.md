`nvs test --mutate` checks whether your tests notice when a line of your program is wrong.

Novis first runs your tests once. Then it makes small changes to your program, one at a time. It
changes `>=` to `>`, `+` to `-` or `true` to `false`, for example. Each changed copy is a mutant.
The tests that run the changed line run again against the mutant. If a test fails, the mutant is
`killed`. If every test still passes, the mutant `survived`. No test checks the result of that line.

The report lists each survived mutant with its file, line and change. The mutation score is the
percentage of mutants that made a test fail. A line that no test runs gives a `not reached` mutant,
and it does not count in the score.

**In plain words:** you hide a small mistake in your code and see if your tests find it.

**Good to know:** every test must pass before the mutants run. A survived mutant does not make
the command fail.

**The examples below** show a mutant that survives, the test that kills it, and code that no test
calls.
