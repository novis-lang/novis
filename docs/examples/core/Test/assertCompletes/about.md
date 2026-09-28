`Core\Test::assertCompletes()` checks that a piece of code finishes within a time limit. You pass the
code as a function with no parameters, and the limit as the option `within`, for example `5s`.

The check works only in a test with a fixed clock, which `#[Test(at: ...)]` sets. It moves that
clock forward by `within` first, and then calls the function. So a loop that waits for a deadline
ends at once, and the test does not wait for real time. Without a fixed clock, the check throws a
`LogicError` and does not call the function.

The check fails with a `Core\Test\Failure` when the function returns but a task it started is still
running. An exception from the function is not changed. The test reports it as it is.

**The examples below** show a job with a limit, the error without a fixed clock, and a test of a
retry loop that ends at a deadline.
