Three endings fire the exit queue, and the report each hook receives says which:

| ending | `reason` | `status` | `error` |
|---|---|---|---|
| the last top-level statement ran | `Normal` | `0` | `null` |
| `exit;`, `exit($n);`, `exit("msg");` | `ExitCall` | `0`, `$n`, `0` | `null` |
| an uncaught `THROWN` reached the root | `UncaughtThrow` | `1` | the `Throwable` |

The queue runs once, at most once per script, and drains until empty. An `exit($n)` puts the very
`$n` on the report rather than a normalised value, and that status arrives with `error` still `null`.

`exit` is the ending this queue exists for: `exit` runs no `finally`, so before the hook existed a
CLI program that had to flush a buffer or write a summary line *whatever the ending* had no home for
that code. A root `try/finally` sees a normal end and an uncaught throw and never sees `exit`, and
taxes every entry file for the coverage it does give.

`ExitReason` is a closed public enum. A future termination kind that should fire the queue is a new
case on this queue, decided on its own; a second queue is the wrong answer.
