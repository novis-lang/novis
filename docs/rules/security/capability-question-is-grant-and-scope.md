At the point of a call the question has exactly two parts: the **grant** — is this capability present
at all in the effective configuration — and the **scope** — does *this argument* fall inside what was
granted: a path under a granted root, a host in a granted list, a binary in a granted set.

Both are answered against the request's own configuration snapshot, cloned once before the program
runs and immutable for the request's whole life. Nothing on the request path re-reads the
configuration tree, so two checks in one request cannot disagree, and a reload between a program's
first syscall and its second cannot widen or narrow what that program may do halfway through.

The decision procedure is pure — a snapshot, a capability, an argument and the run, then a boolean.
The run is whether this is server work and its app folder, the folder of the entry file it started
from or the `root` of the most specific `[[app]]` block over that file. A key the snapshot writes is
the whole grant. A key it does not write takes its default for that run, from the configuration
crate's table of defaults. On a CLI run `fs.read` and `fs.write` default to every path. In server
work `fs.read` defaults to the app folder, less every path with a component below it that starts
with `.` and every `nvs.toml`, and `fs.write` to the directories `Core\IO::temporaryDir` made for
this context, which the context hands the procedure with the run; both of those read too. Every
other default grants nothing. The procedure takes no
context, throws nothing, and is therefore testable without a compiler in front of it. Holding a
capability is not a promise about any one argument; the scope is asked every time.
