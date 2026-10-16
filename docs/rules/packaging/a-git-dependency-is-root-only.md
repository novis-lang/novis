Two kinds of source produce a package's digest, and only one of them is transitive.

A **registry dependency** is a name plus a minimum version, which the registry maps to a digest. A
**git dependency** is a URL plus an **exact commit `rev`** — never a branch, never a tag, because
neither is immutable. The fetched tree is archived and digested exactly as a registry artifact is,
and the digest goes in the lockfile, so from the second fetch onward the two kinds are
indistinguishable.

**Only the root application may declare a git dependency.** A package published to the registry
whose manifest names a git source is rejected at publish time, and a git dependency encountered
below the root is a hard error naming the package that declared it. The convenience of git — a fork,
a patch under test, private code, a monorepo — is entirely a property of *your own* project, and
your transitive graph stays inside a namespace with a transparency log and a retraction mechanism:
no dependency of yours can drag in code from a URL you never saw.

A private registry is a registry: the protocol is static signed files over HTTPS, so an organisation
that cannot use the public one runs or mirrors its own, and nothing above changes.

**Not on disk**, with the rest of the package system.

**Declined:**
- **Git URLs only, with no registry.** Nothing to operate, but the transparency log, retraction, the
  advisory feed and `nvs audit` all need a namespace with an authority behind it, and every URL in the
  transitive graph would be a trust root.
- **A registry only, with git repositories mirrored into it as named packages.** One resolution path,
  but too rigid for private code and monorepos, and not needed: the root-only rule already confines a
  git source to the project that chose it.
