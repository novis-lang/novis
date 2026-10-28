Publishing uses a token scoped to one vendor or one package, which expires after one year at most, starts
with `nvsreg_`, and is stored by the registry only as a hash.

- **The scope** is one vendor or one package, so a token in one project's CI cannot publish another.
- **The expiry** is at most one year, and the owner may choose a shorter one.
- **The `nvsreg_` prefix** lets a secret scanner recognise a leaked token in a commit or a log.
- **The registry stores a hash**, so a copy of its database holds no usable token.
- **`nvs pkg login`** stores the token in the data folder under the ownership check the artifact cache
  uses (`rule:config/ownership-is-the-trust-boundary`).

Creating and revoking a token are log entries (`rule:packaging/the-registry-keeps-an-append-only-log`),
and every owner is notified of a new one (`rule:packaging/an-owner-is-notified-without-email`). A token
only publishes: a release it sends waits seven days
(`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`), and using it never keeps the
account active (`rule:packaging/an-account-stays-active-by-proof-not-by-token`). Everything that changes
ownership, the repository binding or a listing needs a passkey.

**Declined:**
- **A token that never expires.** A token copied into a forgotten CI system stays a publishing key for
  as long as the package exists.

`nvs pkg login` reads the token from standard input, never from an argument, checks it with the
registry's `GET /api/whoami`, and stores it under the data folder's `tokens/`, one file per registry.
`nvs pkg whoami` reads it back only after the ownership check passes, and refuses a file another account
can write. Both take `--registry`, the URL a `[registries]` line names, and `http:` only for a loopback
host (`crates/nvs-cli/src/package/login.rs`). The service side is `apps/registry/src/Tokens.nvs`:
making a token needs a passkey sign-in in the last 15 minutes, and a token stops when its account no
longer owns the vendor.

**Not on disk.** Creating or revoking a token writes no log entry yet and notifies no owner yet.
