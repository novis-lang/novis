A git dependency pins an exact commit, and only the root application may declare one. Two kinds of
source produce a package's digest, and only the registry kind is transitive.

A **registry dependency** is a name plus a minimum version, which the registry maps to a digest. A
**git dependency** is a URL plus an **exact commit `rev`** — never a branch, never a tag, because
neither is immutable. The fetched tree is packed and digested exactly as a registry archive is, and the
digest goes in the lockfile, so from the second fetch onward the two kinds are indistinguishable. The
clone runs over HTTPS inside a size and a time limit, and its host's address is checked against the
outbound address table, so an internal host is refused.

**Only the root application may declare a git dependency.** The registry refuses a publish whose
manifest names one (`rule:packaging/publishing-never-overwrites`), and a git dependency found below the
root is a hard error naming the package that declared it. The convenience of git — a fork, a patch
under test, private code, a monorepo — is a property of *your own* project, and your transitive graph
stays inside registries with a transparency log and retraction: no dependency of yours can bring in
code from a URL you never saw.

**A private registry is a registry.** An organisation that cannot use the public one runs or mirrors
its own, and points its vendors at it (`rule:packaging/a-vendor-resolves-from-one-registry`). Nothing
above changes.

**Declined:**
- **Git URLs only, with no registry.** Nothing to operate, but the transparency log, retraction, the
  advisory feed and `nvs pkg audit` all need an authority behind a name, and every URL in the
  transitive graph would be a trust root.
- **A registry only, with git repositories mirrored into it as named packages.** One resolution path,
  but too rigid for private code and monorepos, and not needed: the root-only rule already confines a
  git source to the project that chose it.

The client clones and packs a root git dependency (`crates/nvs-cli/src/package/git.rs`), and
resolution refuses one found below the root, naming the package that declared it
(`crates/nvs-cli/src/package/resolve.rs`).

**Not on disk.** The registry, which refuses a publish whose manifest names a git dependency, is not
written yet.
