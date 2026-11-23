`Core\Task::all()` runs several callables (functions with no parameters) at the same time and
waits until all finish.

You give them as the fields of an anonymous object, such as `{user: fn() => ..., orders: fn()
=> ...}`. The result has the same field names, and each field has its callable's return type. A page
that calls two web services then waits only as long as the slower one.

If one callable throws an error, the others stop, and the call throws that error. The option `limit`
sets how many callables run at once. The option `deadline` is a time limit for the whole call. When
it runs out, the call throws a `TimeoutError`.

Each callable runs as a task. A task has the same capabilities (rights, such as reading files) and
the same memory limit as the code that calls `all()`. So a script that may only read files has
tasks that may only read files. When a task asks for more memory than the limit, it does not get
the memory, and the program stops with an error.

**Good to know:** the call checks the deadline only while a callable waits, such as for a web
service. So the deadline does not stop a callable that runs an endless loop. The `cpu_time` limit
in `nvs.toml` stops such a loop. In a test with
`#[Test(at: ...)]`, the deadline uses the test's clock, so the test does not wait.
