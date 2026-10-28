An owner may **unlist** a package or one version with a passkey: it is hidden from search and listings,
its page says it is unlisted, and it stays fetchable. Unlisting is a log entry
(`rule:packaging/the-registry-keeps-an-append-only-log`), every owner is notified of it, and an owner
may reverse it.

A lockfile that names an unlisted version keeps building, and resolution still selects it when a
minimum asks for it. Unlisting is not an advisory, so `nvs pkg audit` does not report it
(`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).

Unlisting is the owner's way to say "do not start using this" — an abandoned package, a name published
by mistake — without breaking anyone who already depends on it. A broken or dangerous version is
retracted instead, which is what tells the people who already use it.

**Declined:**
- **Deletion by the owner.** It breaks every lockfile that holds the digest, and a deleted name would
  invite somebody to publish different code under it.

**Not on disk.** There is no registry. The client resolves and fetches an unlisted version a minimum
names, and `nvs pkg add` and `nvs pkg update` never choose one on their own
(`crates/nvs-cli/src/package/minimum.rs`).
