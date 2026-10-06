Novis is for web applications — not a segment, not a vertical, not a trust posture. A team choosing
Novis is choosing it to build the web application they were going to build anyway, and what they get
for it is `tainted` and `secret` checked while compiling rather than scanned afterwards; every
request, job and connection an isolate with an enforceable memory, CPU and time budget; a
dependency's authority declared and narrowed rather than ambient; `decimal`, durations written like `30s` and an
always-valid-UTF-8 `string`; and a reload that swaps a pointer instead of restarting a worker.

**Running code the operator did not write is where those properties pay most, and that is a benefit
rather than a definition.** No document may write as though the untrusted-code case were the point of
the language. An application with one owner, one team and no untrusted input at all is served by
exactly the same properties, because the mistakes they catch are overwhelmingly the application's own.

**Command-line tooling is a second audience, and it stays second.** A team writing internal and ops
tools that touch production credentials, customer data or a shell gets the same purchase from a
program with no HTTP request in it — argv-only process spawning, a `secret` that cannot reach a log,
an interpolated query that does not compile, and a finished tool that is one file with no interpreter
to install. It justifies no new milestone and no reordering; where a slice would serve it at the cost
of a web application's, it loses.

**The safety properties are how Novis is built, not who it is built for.** They are also why this does
not end where a typed, incompatible dialect of an existing language ended before: that one's value depended on a package
compatibility it could not hold, and Novis depends on that compatibility at no point, ships the
framework rather than waiting for an ecosystem to appear, and differentiates on properties an
incumbent cannot acquire in a minor release.
