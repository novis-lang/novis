Running another program is deny-by-default under `process.exec`, the same as every other
syscall-touching entry point: a request may narrow it further and never widen it, and calling without
the grant throws (`rule:security/denial-is-a-runtime-error`). The grant is asked for before the target
is looked at, so an ungranted deployment never learns whether a binary exists.

The capability is only half of it, and the other half is that **there is no shell to interpolate
into**. An executable path and an argv array, with nothing in between that parses a command line,
removes the escaping question rather than answering it — which is a stronger guarantee than any amount
of quoting. Under the sink predicate the executable path is an instruction, so the path is the sink
(`rule:security/sink-predicate`). The argv elements are no sink of their own, and they are still typed
plain `string`, so an argument list holding a `tainted string` does not compile
(`rule:core-classes/process-is-argv-only`): no shell parses an argument, but the program that receives
it does, and an attacker's value in that position can be an option such as `--output=/etc/passwd`.

A grant that names executable roots resolves the same way a spawn root does, canonicalise-then-prefix
(`rule:security/path-scope-canonicalise-then-prefix`).

The target is a path that starts at a root, and never a `PATH` lookup. A bare name such as `git` is
refused before any grant is read (`rule:programs/relative-paths-resolve-from-their-file`), so the
program the operating system starts is the one the check approved — a name a `PATH` entry would have
answered is a program from a directory no grant named.
