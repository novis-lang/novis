Each vendor resolves from exactly one registry, set in `package.toml`, and every vendor not named there
resolves from `https://registry.novis-lang.org`. The client never asks a second registry for a name the
first one lacks.

A registry URL is `https:` or `file:`. `file:` is how the tests and an offline mirror read a directory.
A private registry is an ordinary registry, so an organisation points its own vendors at it and leaves
every other vendor on the default (`rule:packaging/a-git-dependency-is-root-only`).

**`nvs pkg add` and `nvs pkg update` print what a change brings** before a person accepts it: every
capability the package and its whole subgraph request
(`rule:security/package-authority-is-granted-one-line-at-a-time`), the capabilities that changed since
the lock, the number of packages in the subgraph, the owners and repositories that changed since the
lock, and every major a selection crossed (`rule:packaging/resolution-takes-the-highest-minimum`). The
lock records each package's capabilities, repository, commit and owners so the comparison has something
to compare against (`rule:packaging/the-lockfile-holds-every-digest`).

A vendor bound to one registry closes dependency confusion: an attacker who publishes `Acme\Internal`
on the public registry is never consulted for a vendor the application pinned to its own. Searching
registries in order would make the first one that answers the owner of every name.

**Declined:**
- **The first registry that has the name.** That order is exactly how dependency confusion works: the
  public registry answers for a private name it should never have been asked about.

**Not on disk.** The tree has no package system.
