`Core\Test::logs()` returns every log record written since the current test started.

The list contains the records written by the test itself, by each `Core\Test::request` it sends and
by each `Core\Test::run` it starts. They are in the order they were written. Each record is a
`Core\Test\LogRecord` with three methods: `level()`, `message()` and `fields()`. Use it to check
that your code logs the right thing, such as a warning when a login fails.

Each test starts with an empty list, so a record from one test is never in the list of the next
test. Outside `nvs test`, the list is always empty.

**Good to know:** a record below `[log] level` is not written, so it is not in the list. The default
level is `info`, so a `debug` record is not in the list. The records still go to the `[log] target`,
so you also see them in the output of `nvs test`.

**The examples below** show a failed login that writes a warning, the fields of a record that a
request wrote, and a `debug` record that is not kept.
