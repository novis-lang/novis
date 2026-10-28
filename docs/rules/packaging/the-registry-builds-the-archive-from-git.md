The registry builds every archive itself from a git commit: `nvs pkg publish` sends a repository URL,
a commit, a version and the package's directory within the repository, and uploads no file.

- **The directory** defaults to the repository root, and `nvs pkg publish` may name a subdirectory, so
  one repository can hold several packages.
- **The clone** is `nvs registry build`, which fetches that commit over HTTPS from any git host through
  `gix`, inside a time limit and a 512 MiB size limit. It fetches that commit alone, with no history,
  so a repository's past never counts against a package. Its host's address is checked against the
  outbound address table, so an internal host is refused. The clone is larger than the archive limit
  because one repository may hold several packages and the files around them.
- **The archive** is an uncompressed `ustar` tar of the files
  `rule:packaging/a-package-carries-only-its-source-and-what-it-lists` names, sorted by path bytes,
  with every time, owner and mode field fixed and no link or special file. Its BLAKE3 digest is the
  package's identity (`rule:packaging/a-package-is-its-digest`). A transport may compress it.
- **The repository URL is bound at the package's first publish.** Changing it needs a passkey, is a log
  entry, and notifies every owner.
- **`nvs pkg publish --dry-run` runs the same packing locally**, so a publisher can predict the digest
  before sending anything. The client's `nvs pkg pack` and the registry's build run the same code
  (`rule:packaging/the-registry-is-a-novis-program`).

The commit each version was built from is public on the package page, so anybody can read exactly the
source the digest covers. An uploaded file has no such tie: it can contain code that is in no
repository, which is how a release built on a compromised laptop carries what its source does not. A
build the registry does itself makes the public commit and the published bytes one thing.

**Memory:** a build holds one clone, inside its size limit, on disk while it packs. Nothing of it is
held after the digest is recorded.

**Declined:**
- **An uploaded archive.** The bytes would be whatever the publisher's machine produced, with nothing
  linking them to any reviewable source.
- **Binding a package to a forge,** through forge sign-in or OIDC trusted publishing. Any git host
  over HTTPS works, and no forge's account system becomes part of the registry's security
  (`rule:packaging/a-registry-account-is-a-passkey`).
- **A compressed archive as the identity.** Compression output differs between library versions; the
  uncompressed tar is the same bytes on every machine.

**On disk.** `nvs registry build` (`crates/nvs-cli/src/package/build.rs`) clones, packs and digests,
and the service runs it through `Registry\Builder` for every publish (`apps/registry/src/Publish.nvs`).
The service binds the repository URL and the directory at the first publish, and refuses a publish
from another repository.

**Not on disk.** No route changes a package's bound repository with a passkey yet, so a changed
repository is always refused, and no `repository` log entry or owner notice is written for it.
