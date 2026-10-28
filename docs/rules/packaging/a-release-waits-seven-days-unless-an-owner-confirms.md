A release published with a token alone is **pending for seven days**, and an owner's passkey
confirmation on the website releases it at once. The seven days are fixed and have no setting.

- A pending version is **fetchable at once by its exact version** — `nvs pkg add acme/http@1.4.0` — and
  `nvs pkg add` and `nvs pkg update` never select it on their own until the seven days pass
  (`rule:packaging/resolution-takes-the-highest-minimum`).
- **A passkey confirmation** by any owner of the vendor releases it immediately, which is how a hotfix
  ships the same day. The confirmation is a log entry
  (`rule:packaging/the-registry-keeps-an-append-only-log`).
- **A release whose dependency is still pending** waits until that dependency is released.
- **A retraction or an advisory sent with a token alone** waits the same seven days, and one confirmed
  with a passkey is immediate (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).
- **The first release at `0.1.0` or higher** waits for a one-time passkey confirmation that shows the
  exact API being frozen and says that from then on only a major may break it
  (`rule:packaging/only-a-major-may-break-the-api`). Seven days do not release it.

**A public feed lists every pending release**, so anyone can scan them, and every owner is notified of
each one (`rule:packaging/an-owner-is-notified-without-email`). `nvs pkg publish` and `nvs pkg whoami`
print the pending releases the caller did not confirm.

A token is the credential most likely to leak: it sits in CI and on disk. The window turns a stolen
token from a silent release into a public, announced one that every owner sees and anyone can inspect
before a single `nvs pkg update` picks it up. A passkey cannot be copied out of a CI log, so a
confirmation with one is what shortens the window.

**Declined:**
- **A waiting period the owner can set.** Every publisher would set it to zero, and a stolen token's
  release would reach every `update` within the hour.
- **Hiding a pending release until it is released.** A pending release has to be fetchable by exact
  version so it can be tested, and it has to be listed so anybody can check it.

**Not on disk.** There is no registry. The client reads a pending version and the time it becomes
selectable from the index, and `nvs pkg add` and `nvs pkg update` choose it only by its exact version
until then (`crates/nvs-cli/src/package/minimum.rs`).
