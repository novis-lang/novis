There is no `Core\Service`, no new grammar, no runtime change and nothing on the request path. A
service manager is not a stdlib candidate under `rule:core-api/tier-placement`'s tests at all: a Novis
program cannot install itself as a service, only an operator can, from the command line. That keeps
"an application can never grant itself rights" (`rule:security/no-runtime-grant`) intact rather than
restating it.

What the feature costs is paid at start and at stop: one control-handler thread with its stack plus a
status structure per served process — kilobytes, O(1), attributable to no request because no request
causes it — and nothing per request. Its dependencies are the two platform crates the
binary already takes for a signal disposition and a console control handler — one Windows-only, one
Unix-only — both behind `#[cfg]`, both pure Rust with no build script, and neither reachable from a
served request. `sd_notify` adds none of its own: it is a datagram of `NAME=value` lines to whatever
`$NOTIFY_SOCKET` names, and it is written by hand.
