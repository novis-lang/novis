`Core\Test::expectFailure()` runs a function and checks that a check inside it failed. You pass the
code as a function with no parameters. When a check inside it fails, `expectFailure` passes. The
failed checks then do not count against your test.

Inside a test, a failed check counts even when your code catches the `Core\Test\Failure`. So
`try` and `catch` cannot hide a failure. `expectFailure` is the one way to say that a failure is
expected.

When no check inside the function fails, `expectFailure` throws a `Core\Test\Failure`. When the
function throws an error of another class, that error passes through unchanged.

Use it to test a check you wrote yourself. It should pass for good values and fail for bad ones.

**The examples below** show a failure that is expected, what happens when nothing fails, and a test
of your own check for prices.
