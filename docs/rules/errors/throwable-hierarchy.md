The `Throwable` classes are global and are the ordinary `catch` target. They include `ParseError`,
thrown when a file pulled in mid-execution fails to compile — see `rule:errors/compile-failure`.

`Core`'s own tree is small and closed. It has no class for a wrong type or a wrong argument count,
because a program with that mistake does not compile. A domain error is a user-defined class, and
`Core` does not enumerate them. The exceptions are `Core\Db\DbError`, `Core\Db\RolledBack` and
`Core\Cli\NotInteractive`, all three extending `RuntimeError`: a driver failure, a deliberate rollback
and a prompt with no terminal to read have no user-defined home.

The value a resource-limit `FATAL` carries **does not implement `Throwable`**. That is not a
runtime check some future `catch` site could forget: it is a type-checker fact, so `catch
(Throwable $e)` around code that hits a memory or CPU limit provably cannot see the report, the
same way `int + uint` provably cannot compile.

This is what makes "a fatal is not catchable" hold at the ABI level without every call site
cooperating, and it is why no catch-loop can be written for a resource limit at all — not merely
why one is unlikely.
