**`package.lock` records every package in the graph** — direct and transitive — with its name,
selected version, source and digest, and with the requested capabilities, repository, commit and
owners that `nvs pkg update` compares against
(`rule:packaging/a-vendor-resolves-from-one-registry`). `nvs pkg fetch --locked` and
`nvs pkg vendor --locked`, which CI runs, fetch nothing that is not in the lock and fail rather than
update it.

**A digest mismatch is a hard error** — never a warning and never a re-fetch — and the failure names
the package. The digest is the same BLAKE3 the artifact cache keys on, so a package's identity
(`rule:packaging/a-package-is-its-digest`) and its integrity check are one hash.

**`nvs pkg vendor`** writes the whole resolved graph into the application tree, so a build needs no
network at all. Every `nvs pkg fetch` packs each vendored folder again and compares it with the lock's
digest, and writes a folder that differs again from its archive, which makes vendoring a convenience
rather than a second trust root. A build checks nothing against the lock: `vendor/` is committed
source, read as the application's own is.

The lock is for integrity, not for determinism: `rule:packaging/resolution-takes-the-highest-minimum`
is deterministic on its own, and `nvs pkg fetch --locked` on a clean machine writes the same
`vendor/` tree as the machine that wrote the lock.

`nvs pkg fetch` and `nvs pkg vendor` write and check `package.lock` and take `--locked`
(`crates/nvs-cli/src/package/fetch.rs`).

**Declined:**
- **A digest check of `vendor/` on every build.** It reads every vendored file on every compile, and
  it guards nothing that a person who can edit `vendor/` cannot also change in the application's own
  code.
