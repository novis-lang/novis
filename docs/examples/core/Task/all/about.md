`Core\Task::all()` runs several callables at the same time and waits until every one has finished.

You give the callables as the fields of an anonymous object, such as `{user: fn() => ..., orders: fn()
=> ...}`. The result has the same field names, and each field has the type its callable returns. Use it when a page
needs several slow things, such as two web services or a database and a cache. The time the page
waits is then the time of the slowest one, not the total of all of them.

If one callable throws an error, the other callables are stopped, and the call throws that error.
The option `limit` sets how many callables run at the same time. The option `deadline` is a time
limit for the whole call. When it runs out, the call throws a `TimeoutError`. The deadline is checked
only while a callable waits, for example for a web service. An endless loop never waits. The
request's `cpu_time` limit in `nvs.toml` stops it. In a test with `#[Test(at: ...)]`, the deadline uses the test
clock, so the test does not wait for it.

**The examples below** show two results used together, an error from one of the tasks, and a
dashboard loaded with a time limit.
