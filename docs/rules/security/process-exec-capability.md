Running another program is deny-by-default under `process.exec`, the same as every other
syscall-touching entry point: a request may narrow it further and never widen it, and calling without
the grant throws (`rule:security/denial-is-a-runtime-error`). The grant is asked for before the target
is looked at, so an ungranted deployment never learns whether a binary exists.

The capability is only half of it, and the other half is that **there is no shell to interpolate
into**. An executable path and an argv array, with nothing in between that parses a command line,
removes the escaping question rather than answering it — which is a stronger guarantee than any amount
of quoting. Under the sink predicate the argv elements are therefore *data* while the executable path
is an instruction, so the path is the sink and the arguments are not
(`rule:security/sink-predicate`).

A grant that names executable roots resolves the same way a spawn root does, canonicalise-then-prefix
(`rule:security/path-scope-canonicalise-then-prefix`).

The target is a path and never a `PATH` lookup. A bare name resolves against the current directory,
which is the resolution the grant was compared against, so the program the operating system starts is
the one the check approved — a name a `PATH` entry would have answered instead is a program from a
directory no grant named.
