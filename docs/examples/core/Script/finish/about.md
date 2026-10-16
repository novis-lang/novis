`Core\Script::finish()` ends the script at once, from any place in the code. The call does not return,
and no code after it runs. The exit status is `0`, because a script that finished did not fail.

On its way out, every `finally` block between the call and the top of the script runs, so files and
connections are closed. No `catch` block catches it, not even a `catch` for `Throwable`, so error
handling never mistakes it for an error. After the `finally` blocks, the functions added with
`Core\Script::onExit()` run, and their report has the reason `Finish`.

On a server, the response that the handler already set is sent. Work added with
`Core\Task::afterResponse` still runs.

**The examples below** show a job that stops when there is nothing to do, the `finally` blocks that run
on the way out, and a request that is answered at once while the site is under maintenance.
