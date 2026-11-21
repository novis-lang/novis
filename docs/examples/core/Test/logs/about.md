`Core\Test::logs()` returns every log record written since the current test started.

The list contains the records written by the test itself, by each `Core\Test::request` it sends and
by each `Core\Test::run` it starts. They are in the order they were written. Each record is a
`Core\Test\LogRecord` with three methods: `level()`, `message()` and `fields()`. Use it to check
that your code logs the right thing, such as a warning when a login fails.

Each test starts with an empty list, so a record from one test is never in the list of the next
test. Outside `nvs test`, the list is always empty.

**Good to know:** the setting `[log] level` is the lowest level that is written. The default is
`info`. A `debug` record is below it, so it is not written anywhere and is not in the list. Each
record in the list is also written to the `[log] target`, so you see it in the output of `nvs test`.

**The examples below** show a failed login that writes a warning, the fields of a record that a
request wrote, and a `debug` record that is not kept.
