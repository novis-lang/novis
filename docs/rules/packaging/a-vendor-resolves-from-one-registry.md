Each vendor resolves from exactly one registry, set in `package.toml`, and every vendor not named there
resolves from `https://registry.novis-lang.org`. The client never asks a second registry for a name the
first one lacks.

A registry URL is `https:` or `file:`. `file:` is how the tests and an offline mirror read a directory.
A vendor mapped to a registry other than the default names that registry's verifier key beside its URL
(`rule:packaging/the-registry-keeps-an-append-only-log`). The client reads the registry's root file,
`registry.json`, from that URL, and the index and archives from the file hosts the root file lists
(`rule:packaging/registry-hosting-is-configuration`).
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

**Not on disk.** The client resolves each vendor from its one registry
(`crates/nvs-cli/src/package/resolve.rs`), and `nvs pkg add` and `nvs pkg update` print the report
(`crates/nvs-cli/src/package/report.rs`). The verifier key is read and not checked yet.
