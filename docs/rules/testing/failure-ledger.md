`Core\Test\Failure` is an ordinary `Throwable`, and catching one is not merely allowed but
necessary: a composite assertion, a retry wrapper, a soft-assert block and any test *of* an
assertion all need to intercept one.

Every assertion **also** records its outcome into a per-test ledger the test's own code cannot
reach, and the runner reads the ledger rather than the exception state. A body that swallowed its
own failure with `catch (Throwable)` has still failed. There is deliberately no member that reads
the ledger back — that would hand the test the eraser this rule takes away — and the single throw
site records the entry *before* it hands the throw back, so no edge exists on which the throw
happened and the record did not.

`Core\Test::expectFailure(callable)` is the one greppable spelling for a failure consumed on
purpose: it runs the body, requires that it fail, and removes that entry. It decides from the ledger
rather than from what came back, so a body that caught its own failure has still failed; a body in
which nothing failed is itself a failure; and the passing assertions inside a discharged body stay,
because they really ran.
