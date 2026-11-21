Waits for a `Duration`, such as `500ms` or `2s`, and then continues.

A wait of zero or less returns at once and does not throw an error. This is useful when you compute
the time that is left until a deadline, and the deadline has already passed. While one program
waits, the server can run other requests, so a wait does not slow down other users. A wait can be
cancelled, for example because a deadline passed. Then the code stops at the `sleep` call.

In a test with a fixed clock, `#[Test(at: ...)]`, a sleep does not wait. It ends at once, and the
test clock moves forward by its length.

**The examples below** show a retry that waits longer after each failure, a wait of zero or less,
and a pause between batches of messages.
