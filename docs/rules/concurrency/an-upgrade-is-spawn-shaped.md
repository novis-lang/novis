`Core\Socket::upgrade('sockets/chat.nvs', args: {room: $room})` takes a spawned script's operand and
nothing else: a file path, resolved and root-checked exactly as that construct's is, or a static
method written `Chat::run(...)` whose parameters `args:` binds to **by name**. Which of the two it is
is decided syntactically at the call site (`rule:security/isolate-shares-nothing`).

It is never a callable. A capture would carry state across the boundary the isolate exists to create,
so an anonymous function or a `callable`-typed variable here is a compile error that names the method form
instead. Naming compiled code is also what makes a connection participate in the artifact cache, hot
reload, grants, limits, coverage and tracing with no special case in any of them.

`args` crosses by graph copy (`rule:classes/graph-copy`), so it is a value and never a shared
reference: a `secret` may not be passed at all, and a `tainted` value stays `tainted` on the other
side.

Calling it is what performs the upgrade, and it answers `void`. Nothing in this language reads a
handler's return value, so an upgrade handed back to the runtime would be handed to nobody.
