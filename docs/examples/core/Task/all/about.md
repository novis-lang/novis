`Core\Task::all()` runs several closures at the same time and waits until every one has finished.

Each closure is one field of a shape. The result has the same field names, and each field has the
type its closure returns. Use it when a page
needs several slow things, such as two web services or a database and a cache. The time the page
waits is then the time of the slowest one, not the total of all of them.

If one closure throws an error, the other closures are stopped, and the call throws that error.
The option `limit` sets how many closures run at the same time. The option `deadline` is a time
limit for the whole call. When it runs out, the call throws a `TimeoutError`.

**The examples below** show two results used together, an error from one of the tasks, and a
dashboard loaded with a time limit.
