- **`ChannelBinding::unrequested()` cannot authenticate against a TLS-enabled PostgreSQL, and it is
  the spelling that reads as correct.** `unrequested` is the gs2 header `y,,`, which asserts the
  server offers no channel binding, and a server offering `SCRAM-SHA-256-PLUS` — every SSL
  connection — reads it as a downgrade and fails with "channel binding check failed". Use
  `unsupported` (`n,,`); `crates/nvs-db/src/pg.rs`'s module doc holds the reasoning.
  [until: reviewed 2026-09-06]
