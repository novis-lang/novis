`Core\Test::advance()` moves the clock of a test forward by a duration. It works only in a test that
fixes its clock with `#[Test(at: ...)]`. After the call, `Core\Time::now()` returns the moved time.

Use it to test code that depends on time. Examples are a login that ends after 30 minutes, or a
link that stops working after a day. The test does not wait: it moves the clock and checks the result at once.
Every test has its own clock, so moving it in one test does not change any other test. A
`Core\Time::sleep` in that test does not wait either. It moves the same clock forward.

A negative duration moves the clock back. In a test without `at:`, or outside a test, `advance`
throws a `LogicError`. The clock can show a time between the years -9999 and 9999. When the moved
time is outside that range, `advance` throws a `RuntimeError` and the clock does not move.

**The examples below** show a token that expires and a clock moved in several small steps. They
also show a session that ends after 30 minutes without use, and a retry that waits longer after
each failure.
