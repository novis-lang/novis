The `Throwable` classes are global and are the ordinary `catch` target. They include `ParseError`,
thrown when a file pulled in mid-execution fails to compile — see `rule:errors/compile-failure`.

The value a resource-limit `FATAL` carries **does not implement `Throwable`**. That is not a
runtime check some future `catch` site could forget: it is a type-checker fact, so `catch
(Throwable $e)` around code that hits a memory or CPU limit provably cannot see the report, the
same way `int + uint` provably cannot compile.

This is what makes "a fatal is not catchable" hold at the ABI level without every call site
cooperating, and it is why no catch-loop can be written for a resource limit at all — not merely
why one is unlikely.
