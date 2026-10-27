The application's grant table is keyed on namespace prefixes and is the `[grants]` table of the
application's own `package.toml` (`rule:packaging/package-toml-is-a-tool-input`), never a package's
manifest. A package still declares what it requests, and that declaration still grants nothing; the
application grants explicitly, one line at a time
(`rule:security/package-authority-is-granted-one-line-at-a-time`).

A capability named in a grant line must appear on the roster
(`rule:security/capability-roster-is-closed`); an unknown name is an error when the table is read,
because a directive that silently means nothing is worse than one that refuses.

The effective set at a call site is the **intersection** of the operator's configuration in `nvs.toml`,
the application's grant, the package's own declaration and any narrowing an enclosing isolate applied —
every one of which may only tighten (`rule:security/no-runtime-grant`). The operator's `nvs.toml` can
therefore narrow what `[grants]` gives and never widen it.

The key is a namespace because a path is the one thing about third-party code a developer is invited
to change. Overriding one class of a dependency by putting a file in an earlier autoload root is a
supported pattern, and under a path key that file would hold the application's authority. Code the
package manager never fetched, such as a git submodule, a `discover` glob or a pasted file, has no
package path at all, and most untrusted code arrives that way. A namespace is stable exactly where the
path is not: an override must declare the namespace it overrides, and hand-copied code keeps the
namespace its own source declares.

**For a registry package the grant table and the package graph are the same thing.** A package's name
is its root namespace (`rule:packaging/a-package-name-is-vendor-slash-name`), and every file in it stays
under that namespace (`rule:packaging/a-package-file-stays-under-its-declared-prefix`). A name has two
segments, so no other package lives under `Acme\Http`, and the line `nvs pkg add` writes for it covers
exactly that package's code. The same table
also covers code no package manager fetched, by the namespace that code declares.

`Core` is never a grantee. The namespace `Core` and everything under it is reserved
(`rule:core-api/reserved-namespace`), so a key under `Core` is an error and no grant line can name a
namespace that inherits the standard library's own authority.

**Memory:** the grant table is compile-time input, and nothing of it is held per request.

**Declined:**
- **A key on the package rather than the namespace, with the override diagnosed.** It closes the
  override and leaves hand-copied code, which never had a package, unprotected, and it makes a
  supported pattern argue with its author.
- **A `grants` clause on the `autoload` declaration.** A `require`d file may declare a namespace with
  no `autoload` line involved, so grants on the import site protect only code that arrived by that
  route. For a fetched package the clause would also sit in a generated file marked *do not edit*,
  it would state a fact `[grants]` already states, and one `discover` line covers many prefixes, so
  the clause has no single namespace to attach to.