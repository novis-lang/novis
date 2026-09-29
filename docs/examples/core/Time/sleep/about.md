Waits for a `Duration`, such as `500ms` or `2s`, and then continues. It replaces PHP's `sleep`,
`usleep`, `time_nanosleep` and `time_sleep_until`, because the `Duration` already says which unit
you mean.

A wait of zero or less returns at once and does not throw an error. This is useful when you compute
the time that is left until a deadline, and the deadline has already passed. While one program
waits, the server can run other requests, so a wait does not slow down anybody else. If a task is
cancelled while it waits, for example because a deadline passed, it stops there.

**The examples below** show a retry that waits longer after each failure, a wait of zero or less,
and a pause between batches of messages.
