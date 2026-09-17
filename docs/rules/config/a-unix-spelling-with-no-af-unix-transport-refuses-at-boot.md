A Unix spelling on a platform this build has no Unix transport for is refused **where the key was
written**, at boot, with a note naming the platform and pointing at loopback TCP. The Unix transport
is `#[cfg(unix)]`, and there is no Windows fallback because `AF_UNIX` exists there but the reactor's
I/O layer does not carry it.

At boot rather than at run time, which is the opposite of the answer a session `backend = "db"` gets
(`rule:core-api/session-roster`), and the difference is what each is waiting for. `db` names a store
the roster admits whose second half is unwritten, so refusing it at the key would claim the decision
was wrong rather than that the build has not caught up. A Unix socket on a platform with no `AF_UNIX`
is not waiting for this project — the deployment has to be spelled differently, and every request
until someone notices is one a boot refusal would have prevented. So it sits with `backend = "local"`:
refused where it is written, so a deployment cannot run believing it has a store it will never reach.

**Silently reading it as loopback TCP is refused.** A configuration that reads as one transport and
runs as another is worse than one that does not run, because the difference is invisible in exactly
the review that would have caught it.
