`message()` returns the message of a log record. In a test, `Core\Test::logs()` returns one
`Core\Test\LogRecord` for each record, and `message()` is one of its methods.

The result is the `$message` string that your program gave to `Core\Log::write`, with no change.
The level and the extra fields are not part of it. Use `level()` and `fields()` to read them.

Use `message()` to check that your code logs the right event. A record is easy to find in a list
when its message names what happened, such as `"user deleted"`.

**Good to know:** keep values that change, such as a user name or a count, in the fields. A short
message that never changes is easier to test and easier to search for in a log.

**The examples below** show an audit record for a deleted user, a nightly import that logs one
message for each step, and a search request with no search words.
