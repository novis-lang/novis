The `Throwable` classes are global and are the ordinary `catch` target. They include `ParseError`,
thrown when a file pulled in mid-execution fails to compile — see `rule:errors/compile-failure`.

`Core`'s own tree is small and closed. It has no class for a wrong type or a wrong argument count,
because a program with that mistake does not compile. A domain error is a user-defined class, and
`Core` does not enumerate them. The exceptions are `Core\Db\DbError`, `Core\Db\RolledBack` and
`Core\Cli\NotInteractive`, all three extending `RuntimeError`: a driver failure, a deliberate rollback
and a prompt with no terminal to read have no user-defined home.

A class joins the tree only with `extends`. `implements` names an interface, so `implements
RuntimeError` is `E0303` as `implements` of any class is, and so is `extends` of a reserved interface.

Only the tree is thrown and caught. A `throw` of a value whose type is not a class of the tree is
`E0780`, an anonymous object and a shape included, since no `catch` matches their class. `mixed` and
`object` are allowed, and a `null` beside a class of the tree is too. A `catch` naming a type no
`Throwable` can be is `E0864`, since it never runs: a scalar, an array, an enum or a class outside the
tree. An interface is allowed, since a class of the tree may implement it, and so are `object` and
`mixed`.

The value a resource-limit `FATAL` carries **does not implement `Throwable`**. That is not a
runtime check some future `catch` site could forget: it is a type-checker fact, so `catch
(Throwable $e)` around code that hits a memory or CPU limit provably cannot see the report, the
same way `int + uint` provably cannot compile.

This is what makes "a fatal is not catchable" hold at the ABI level without every call site
cooperating, and it is why no catch-loop can be written for a resource limit at all — not merely
why one is unlikely.
