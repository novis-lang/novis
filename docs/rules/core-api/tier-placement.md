A candidate for the standard library is placed by six tests, applied in order, rather than by where another
library happened to put it:

1. **Does it need runtime privilege?** Direct heap access, the request lifecycle, the compiler's own
   tables, or state that outlives a request. This disqualifies every stateful client — a connection pool, a
   broker consumer, a bound directory session, a cached WSDL — because a sandboxed guest is re-instantiated
   per request and loses it every time.
2. **Is it an injection sink or a launderer?** A launderer is always Core, because only a `Core` member
   whose contract names a sink may remove a qualifier (`rule:core-api/qualifier-behaviour-is-declared`).
3. **Does it wait on the outside world?** Sockets, files, child processes, timers. This forces a capability
   grant and, with test 1, usually forces a natively linked subsystem.
4. **Is per-call cost near call overhead?** A string-length call cannot pay a boundary crossing; a
   microsecond of formatting comfortably can.
5. **Does it parse hostile bytes?** Image codecs, archive readers, document parsers. This is the sandbox's
   headline case rather than its consolation prize.
6. **Would two of these exist?** One spelling per job, applied to the library instead of the syntax.

The costs these tests weigh are **API surface** and the **unsandboxed dependency set** — not binary size,
and emphatically not runtime memory, which for an uncalled `Core` class is zero. "Do not bloat the core"
therefore reads as "do not grow the public surface, and do not grow the code holding the process's
authority". Coroutine suspension is not a constraint: a host import can suspend its caller, so an
I/O-bearing component is possible in principle, and the database and cache clients are native because of
connection lifetime rather than because of blocking.
