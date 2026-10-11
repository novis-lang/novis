The application's grant table is keyed on namespace prefixes and lives in the application's own
configuration rather than in a package manifest. A package still declares what it requests and that
declaration still grants nothing; the application grants explicitly, one line at a time.

A capability named in a grant line must appear on the roster
(`rule:security/capability-roster-is-closed`); an unknown name is an error at boot, because a
directive that silently means nothing is worse than one that refuses.

The effective set at a call site is the **intersection** of the operator's configuration, the
application's grant, the package's own declaration and any narrowing an enclosing isolate applied —
every one of which may only tighten (`rule:security/no-runtime-grant`).

The key is a namespace because a path is the one thing about third-party code a developer is invited
to change. Overriding one class of a dependency by putting a file in an earlier autoload root is a
supported pattern, and under a path key that file would hold the application's authority. Code the
package manager never fetched, such as a git submodule, a `discover` glob or a pasted file, has no
package path at all, and most untrusted code arrives that way. A namespace is stable exactly where
the path is not: an override must declare the namespace it overrides, and hand-vendored code keeps
the namespace its own source declares.

`Core` is never a grantee. The namespace `Core` and everything under it is reserved
(`rule:core-api/reserved-namespace`), so no grant line can name a namespace that inherits the standard
library's own authority.

**Memory:** the grant table is compile-time input, and nothing of it is held per request.

What this gives up is that the grant table and the package graph are no longer the same thing. One
package may declare several prefixes, and hand-vendored code has a prefix and no package, so the line
written for a package does not promise to cover exactly that package's code. That is the price of
covering code no package manager fetched, which is the code a package key left holding everything.

**Declined:**
- **The package key kept, with the override diagnosed.** It closes the override and leaves
  hand-vendored code, which never had a package, unprotected, and it makes a supported pattern argue
  with its author.
- **A `grants` clause on the `autoload` declaration.** A `require`d file may declare a namespace with
  no `autoload` line involved, so grants on the import site protect only code that arrived by that
  route. For a fetched package the clause would also sit in a generated file marked *do not edit*,
  it would state a fact `[grants]` already states, and one `discover` line covers many prefixes, so
  the clause has no single namespace to attach to.

**Not on disk.** The tree reads the deployment's capability block; it has no per-namespace grant
table, and the diagnostics this rule needs do not exist.
