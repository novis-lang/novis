Hardcoded in Rust, with no script execution — there is nothing left here that could throw, panic or
need a further tier, because this is the floor. It writes to an operator-owned sink named by `[log]
target`: `stderr`, `file:<path>` or `syslog`.

It fires when `[log] handler` is unset or `rule:errors/handler-script` failed for any reason. **If
even this write fails — a full disk, a broken pipe — the failure is swallowed**, and the request
tears down normally regardless. That is deliberately boring: an undefined "what then" at the true
floor is worse than a defined "give up".

Because it writes unconditionally, it is bounded against the disk it writes to. A file target
rotates under a retention bound, and repeated identical records inside a window coalesce into one
record carrying a `count`. Both bounds sit on the sink, so no caller has to be trusted to be rare.

**It also restores the terminal**, before it writes and before it gives up. A CLI program holding
raw mode, a hidden cursor or a live region has put the operator's shell into a state only this
ladder can leave; a `finally` is not enough, because an internal panic bypasses user code by
design. A ladder that protects the process and leaves the shell unusable has failed at what it is
for.
