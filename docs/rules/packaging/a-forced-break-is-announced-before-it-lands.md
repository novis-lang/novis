Where a deprecation cycle is possible — meaning the old and new behaviour can coexist for one
release — the old spelling survives one **minor** release emitting a **compiler warning that names
its replacement**, and is removed in the next **major**. A warning that does not name the
replacement is not a deprecation, it is an inconvenience.

Where a cycle is impossible — a security fix, or behaviour that cannot be dual-run — the change
ships immediately with a release note that says plainly what changed, on whatever release ships
soonest (`rule:packaging/who-can-see-it-decides-the-release-slot`'s last row). Silence is never an
option in either case.

This is the seventh step of `rule:packaging/a-dependency-break-is-absorbed-never-forwarded`, and it
is reached only after the six above it are exhausted.
