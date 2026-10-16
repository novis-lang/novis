The error a failed check throws.

Every check `Core\Test` offers does two things when it does not hold: it throws this, and it records
the outcome in a ledger for the test it is in. The runner reads the ledger rather than the throw, so
a test that catches its own failure and carries on has still failed, and nothing it can call will
say otherwise.

That is what makes catching one safe rather than a loophole. A helper that checks every field of a
record and reports all the wrong ones at once, a block that keeps going after the first
disappointment, a test written about a check of your own: each of those has to intercept a failure,
and none of them can hide one. `Core\Test::expectFailure` is the single spelling for a failure
consumed on purpose.

**Good to know:** it hangs off the root of the error tree directly rather than under the errors a
running program raises, so a `catch` written for those never picks one up by accident.
