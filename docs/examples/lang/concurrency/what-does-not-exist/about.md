PHP's concurrency tools do not exist in Novis. Naming one is a compile error, and the message names
what to write instead.

There is no `async` keyword, no `await` on a promise, no `Fiber`, and no `pcntl_fork`, `pthreads`,
`parallel` or `curl_multi_init`. A method that waits just waits, and the code around it is ordinary
code. Run several jobs at the same time with `Core\Task::all`, or one job per element with
`Core\Task::map`. Run a whole file or a static method beside your program with `spawn script`, and
read its result with `await`. There is no `spawn worker`, and an anonymous function is not allowed
as the entry.

Nothing is shared between requests, or between a spawned file and your program. `global`,
`$GLOBALS` and a `static` variable inside a function are compile errors. Values cross at `await`.

**The examples below** show `Core\Task::all` in place of promises, `spawn script` in place of
`pcntl_fork`, and a page that asks three shops for a price at the same time.
