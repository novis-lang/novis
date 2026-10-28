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

An owner signed in with a passkey in the last 15 minutes unlists a version, or every version, with
`POST /api/packages/<vendor>/<name>/unlist` and reverses it with `.../relist`
(`apps/registry/src/App.nvs`). Each change is an `unlist` or `relist` log entry saved with it, and the
index file shows the version as `unlisted`. A token cannot unlist: the request is answered `202` with
the page where an owner confirms it, and `nvs pkg unlist` prints that page
(`crates/nvs-cli/src/package/unlist.rs`). The client resolves and fetches an unlisted version a
minimum names, and `nvs pkg add` and `nvs pkg update` never choose one on their own
(`crates/nvs-cli/src/package/minimum.rs`).

**Not on disk.** There is no search, no package page and no owners' notification yet, so nothing
hides an unlisted package from a listing, and no owner is told of an unlist.
