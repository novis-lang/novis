`ProcessOptions` carries three fields and no more: a working directory, an environment, and a
timeout. It is one options bag, `{cwd?: string, env?: array<string>, timeout?: Duration}`, and
`Core\Process::run` and `::spawn` both take it, because there is no reason for one of them to take a
working directory the other does not.

`cwd` is a path position: a relative path given as a string literal is joined to the folder of the file that wrote it, and a
relative value built at run time is refused, because the server's own working directory names a
different place depending on how it was started. It needs no capability of its own — the child is the
program `process.exec` approved, and it can change its own folder the moment it runs.

`env`, when given, **replaces** the child's environment entirely rather than merging with the
parent's — explicit replacement is simpler to reason about than merge semantics. Every key and value
is plain `string`, so an API key held as a `secret` needs `rule:core-classes/secret-reveal` first;
this is a new sink reusing an existing escape hatch, not a new mechanism. A name that is empty or holds
`=` or a NUL, and a value that holds a NUL, are refused before anything starts.

`timeout` counts from the start of the child, and is no timer of its own: it bounds the waits the two
members already park on. `run`'s wait for the exit and every `Core\Process\Handle` member that parks —
a read, a write, `wait` — stop waiting when it passes, kill the child, and resume the suspended
coroutine into a `TimeoutError` naming the member. A handle whose timeout has passed throws the same
error from every later member but `kill`. A timeout that is not a positive length of time is a
`RuntimeError` before anything starts, as `Core\Net\Listener::accept`'s bound is.
