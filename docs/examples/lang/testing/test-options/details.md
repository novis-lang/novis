The options are `skip: string`, `at: string`, `seed: int`, `retries: int`, `because: string`,
`db: string`, `server: bool`, `timeout: string` and `tags: array<string>`. An option with another name is `E0454`, and its help lists them. A
value of the wrong type is `E0401`, so `skip: true` does not compile. An option given twice is
`E0304`.

A skipped test is not built, so its constructor does not run. The fixtures only it needs are not
built either. The report prints the reason after `skipped:`.

`at:` takes a time in the RFC 3339 format, such as `"2026-01-01T00:00:00Z"`. `Core\Time::now()`
returns that time in the test. A text in another format compiles, and the test fails with a message
that shows the format.

`Core\Test::advance` moves the fixed clock forward by a duration, such as `90m`. In a test without
`at:`, it throws a `LogicError`. It throws the same error under `nvs run`. The clock of the computer
never moves.

With `seed:`, `Core\Random` and `Core\Uuid::v7` give the same values on every run. Two tests with the
same seed get the same values. A test without `seed:` gets different values on every run.

`retries:` without `because:` is `E0734`. `because:` alone compiles and changes nothing.

A test that fails and then passes is reported as flaky. The mark is `!`, and the report prints the
message of the failed attempt. A test that fails every attempt is reported as failed, with the
message of the last attempt.

`db: "main"` opens a transaction on the database connection `[db.main]` before the test, and
rolls it back after the test. The test sees the database as it was, and leaves no change in it. If no
`[db.main]` block is configured, the test fails with a message that says so.

`server: true` starts a real HTTP server for the test, on a free port of the local computer.
`Core\Test::serverUrl()` returns its address. In a test without `server: true`, it returns `null`.
