`nvs pkg publish` names a repository, a commit and a version, and the registry never overwrites a
published `name@version`. A version, once published, is immutable; a bad one is retracted
(`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).

The caller authenticates with a publish token (`rule:packaging/a-publish-token-is-scoped-and-expires`),
and the registry builds the archive itself from that commit
(`rule:packaging/the-registry-builds-the-archive-from-git`). **It refuses the publish when:**

- a file declares a namespace outside the package's name
  (`rule:packaging/a-package-file-stays-under-its-declared-prefix`), or the name differs only by case
  from one already taken (`rule:packaging/a-package-name-is-vendor-slash-name`);
- the archive holds an `nvs.toml` (`rule:packaging/a-package-cannot-reach-its-host`);
- the manifest names a git dependency (`rule:packaging/a-git-dependency-is-root-only`);
- the clone is over its size limit (`rule:packaging/the-registry-builds-the-archive-from-git`);
- the archive breaks `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`: it is over
  16 MiB or 5,000 files, has no license file, has a path another platform cannot hold, or `files`
  names a missing file, a `.nvs` file or one the archive already carries;
- a `[[download]]` entry is malformed
  (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`);
- a minor or patch breaks the API of the release before it
  (`rule:packaging/only-a-major-may-break-the-api`);
- the version was published before, under any circumstance;
- the account is not active (`rule:packaging/an-account-stays-active-by-proof-not-by-token`), or has
  no verified notification channel (`rule:packaging/an-owner-is-notified-without-email`).

A publish that passes is recorded in the transparency log
(`rule:packaging/the-registry-keeps-an-append-only-log`) **before** it is downloadable, together with
the manifest's requested capabilities, so that `nvs pkg add` shows them before a person grants
anything (`rule:security/package-authority-is-granted-one-line-at-a-time`). A release by token alone
then waits seven days unless an owner confirms it
(`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`).

An immutable version is what makes a digest in a lockfile mean the same thing next year, and what
lets a mirror or a `vendor/` copy be checked against the log instead of trusted.

**Declined:**
- **Overwriting a version to fix a broken release.** Every lockfile that holds the old digest stops
  building, and a registry that may change bytes under a name has made the log's promise false.

**On disk.** `nvs pkg publish` sends the commit, and the service's `POST /api/publish` is
`Publish::publish` (`apps/registry/src/Publish.nvs`). It runs `nvs registry build`, which refuses
everything one commit can show, and then refuses a version published before, an inactive account, a
name outside the token's scope or the vendor's owners, a name that differs only by case, and a changed
repository. `Index::catchUp` (`apps/registry/src/Index.nvs`) appends the `publish` entry, whose
digest covers the manifest and its capabilities, before it writes and signs the index file that
lists the version and its capabilities. The publish is answered only after that.

**Not on disk.** The refusal for an account with no verified notification channel is not checked
yet, because no channel exists yet.
