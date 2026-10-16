Four endings fire the exit queue, and the report each hook receives says which:

| ending | `reason` | `status` | `error` |
|---|---|---|---|
| the last top-level statement ran | `Normal` | `0` | `null` |
| `exit;`, `exit($n);`, `exit("msg");` | `ExitCall` | `0`, `$n`, `0` | `null` |
| an uncaught `THROWN` reached the root | `UncaughtThrow` | `1` | the `Throwable` |
| `Core\Script::finish()` was called | `Finish` | `0` | `null` |

`Core\Script::finish()` ends the script, or the response, from any frame as an **ordinary** end. It
travels the throw path, because that is where every `finally` lives, so every `finally` between the
caller and the root runs on the way out. No `catch` sees it: the class it raises,
`Core\Script\Finished`, is a root of its own and not a `Throwable`, and a `catch` naming it is
`E0814`. At the root it is not a failure, so the on-uncaught-throw handler does not run, and on the
request path `afterResponse` work drains as it does after a normal end.

`finish()` exists because `exit` is the only other way to stop early, and `exit` runs no `finally` and
hands `afterResponse` work back unrun. A program that has written its answer and still has cleanup to
run would otherwise reach for `echo $body; exit;`, a line that reads like a return and silently drops
both halves of that cleanup. Running every `finally` is the whole difference between the two endings:
one that reaches the root without running them is `exit` under another name. The member is on
`Core\Script`, beside `onExit`, because the moment it brings forward is the request task's own frame
returning, which is the end of the response under a server and the end of the script under `nvs run`.
A program that never calls it pays nothing, and one that does pays for the unwind a `throw` already
pays for.

The queue runs once, at most once per script, and drains until empty. An `exit($n)` puts the very
`$n` on the report rather than a normalised value, and that status arrives with `error` still `null`.

`exit` is the ending this queue exists for: `exit` runs no `finally`, so before the hook existed a
CLI program that had to flush a buffer or write a summary line *whatever the ending* had no home for
that code. A root `try/finally` sees a normal end and an uncaught throw and never sees `exit`, and
taxes every entry file for the coverage it does give.

`ExitReport::Reason` is a closed public enum. A future termination kind that should fire the queue is a new
case on this queue, decided on its own; a second queue is the wrong answer.

**Declined:**
- **An `exit` that runs `finally` when the host is a server.** One keyword would mean two things, and
  which one it meant would be a property of the deployment
  (`rule:statements/exit-is-the-only-termination-keyword`).
- **The ending as a new status beside `THROWN`.** A call site's landing block runs `finally` only for
  a thrown status, so a new status would skip every `finally`, which is the one property this ending
  exists to have.
- **`Core\Response::finish()`.** It would mean nothing under `nvs run` and need a second name there
  for the same moment.
