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

The service side is `apps/registry/src/Releases.nvs`. A publish stores the release with its seven
days, and `Releases::selectableAt` takes the later of its own time and every dependency's, walking at
most 16 steps; a dependency outside the registry never waits. The first release at `0.1.0` or higher,
and a later stable one while no release at or below it was confirmed, has no time at all, and the
index writes it as `{"pending": {}}`. An owner signed in with a passkey in the last 15 minutes
releases a version with `POST /api/packages/<vendor>/<name>/<version>/release` and freezes the first
stable API with `.../stable`, whose page reads the API from `.../api`. Each confirmation is a
`release-confirmed` or `stable-api-confirmed` log entry saved with the change (`Events.nvs`). A
retraction sent with a token shows seven days later (`App::retract`), and an advisory sent with a
token enters the log and the feed seven days later (`App::advise`). The client reads both pending
forms from the index, and `nvs pkg add` and `nvs pkg update` choose a pending version only by its
exact version (`crates/nvs-cli/src/package/minimum.rs`).

The signed public list of pending releases is `GET /feeds/pending` (`Feeds::pending`,
`apps/registry/src/Feeds.nvs`), and each pending release notifies the vendor's owners. `nvs pkg
publish` and `nvs pkg whoami` print the account's pending releases
(`crates/nvs-cli/src/package/login.rs`).

**Not on disk.** The website has no page where an owner confirms a release or a stable API with a
passkey (gap:nvs-cli/release-confirmation-pages).
