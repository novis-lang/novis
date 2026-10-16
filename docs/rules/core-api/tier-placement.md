A candidate for the standard library is placed by six tests, applied in order, rather than by where another
library happened to put it. PHP's extension list tracks how its C modules were built and loaded —
separate shared objects, load order, per-module globals — and adopting it would import its duplications
with it: two MySQL APIs, four XML APIs, SHA-256 in two extensions, and `iconv` beside `mbstring`.

1. **Does it need runtime privilege?** Direct heap access, the request lifecycle, the compiler's own
   tables, or state that outlives a request. This disqualifies every stateful client — a connection pool, a
   broker consumer, a bound directory session, a cached WSDL — because a sandboxed guest is re-instantiated
   per request and loses it every time.
2. **Is it an injection sink or a launderer?** SQL text, HTML output, headers, filesystem paths, argv,
   logs (`rule:security/tainted-qualifier`). A launderer is always Core, because only a `Core` member
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

The cost the tests accept is a default binary whose unsandboxed dependency set is larger than a minimal
runtime's: the database drivers, the HTTP client and the crypto primitives are all in it. Each is audited under `deny.toml`, and each is there because test 1 or test 2 put it there.

**Declined:**
- **PHP's extension list, one for one.** It imports every duplication above, and its partition encodes
  C build constraints Novis does not have.
- **Everything in Core, with no tiers for first-party code.** Image and archive decoders, the code that
  turns an upload into code execution, would run in-process with every in-flight request, and the
  unsandboxed dependency set would have no bound.
- **A minimal core with everything else a package.** A launderer or an injection sink cannot be
  third-party (`rule:security/launderers-are-sink-named`), and leaving HTTP, JSON, crypto and dates to
  an ecosystem produces several incompatible implementations of each.
