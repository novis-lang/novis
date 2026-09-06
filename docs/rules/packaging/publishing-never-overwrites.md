`nvs publish` uploads a source archive built from a **clean checkout**. The registry requires a
verified account with a second factor, and then:

- **refuses to overwrite a published `name@version` under any circumstance** — a version, once
  published, is immutable; a bad one is retracted
  (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`);
- refuses an archive containing an `nvs.toml` (`rule:packaging/a-package-cannot-reach-its-host`) or
  a git dependency (`rule:packaging/a-git-dependency-is-root-only`);
- records the artifact in the transparency log
  (`rule:packaging/the-registry-keeps-an-append-only-log`) **before** it is downloadable;
- records the manifest's requested capabilities, so that `nvs add` can show them before a human
  grants anything (`rule:security/package-authority-is-granted-one-line-at-a-time`).

Operating the registry — index, artifact store, log, advisory feed, accounts, moderation, uptime —
is a real and permanent responsibility; the protocol is static signed files so that the serving side
is cheap and mirrorable, but the responsibility is the main cost of having a package system at all.
