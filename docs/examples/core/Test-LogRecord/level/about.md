`level()` returns the level of a log record, in lower case. In a test, `Core\Test::logs()` returns
one `Core\Test\LogRecord` for each record, and `level()` is one of its methods.

The result is one of five strings: `"debug"`, `"info"`, `"warn"`, `"error"` or `"critical"`. It is
the same text as the `level` in a JSON log line. It matches the `Core\Log::Level` case that your
program gave to `Core\Log::write`.

Use `level()` to check that a problem is logged at the right level. A failed payment should be an
`error`, not an `info`.

**Good to know:** a record below the setting `[log] level` is not kept. The default is `info`, so a
test never sees a `debug` record unless the setting is lower.

**The examples below** show a failed payment, a disk that fills up step by step, and a request with
a wrong API key.
