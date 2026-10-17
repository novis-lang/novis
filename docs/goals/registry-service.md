---
looper: 1
title: The registry is a Novis application that builds every release from git
summary: "apps/registry is a Novis web application with passkey accounts, recovery codes and a proof-of-work sign-up, owned vendors, scoped expiring tokens, publishing that builds the archive from a git commit and refuses what its rules refuse, a seven-day pending window an owner's passkey ends early, unlisting, withholding, a log entry per event, owner notifications by Web Push, webhooks, feeds and an inbox with no email and a verified channel required to publish or become an owner, a 60-day activity rule that token use never satisfies, counted downloads and the static index; nvs pkg login, whoami, publish, retract and unlist work against it, it has a deploy guide, the commands have their feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [apps/registry/README.md, apps/registry/src/App.nvs, crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/the-registry-is-a-novis-program.md]
stages:
  - id: 1
    title: the application exists, and an account is passkeys, recovery codes and a proof-of-work sign-up
    status: open
    context: [apps/notes/src/App.nvs, apps/notes/README.md, crates/nvs-stdlib/src/passkey.rs, docs/rules/packaging/a-registry-account-is-a-passkey.md]
    extra:
      context:
        rules: [packaging/the-registry-is-a-novis-program, packaging/a-registry-account-is-a-passkey, core-classes/passkey-verifies-a-webauthn-ceremony]
  - id: 2
    title: a vendor has owners, and a token is scoped, hashed and expires
    status: open
    context: [crates/nvs-config/src/data.rs, crates/nvs-cli/src/cache.rs, docs/rules/packaging/a-publish-token-is-scoped-and-expires.md, docs/rules/packaging/a-package-name-is-vendor-slash-name.md]
    extra:
      context:
        rules: [packaging/a-publish-token-is-scoped-and-expires, packaging/a-package-name-is-vendor-slash-name, config/ownership-is-the-trust-boundary]
  - id: 3
    title: a publish builds the archive from a git commit, refuses what the rules refuse, and lands in the static index
    status: open
    context: [crates/nvs-cli/src/package/pack.rs, crates/nvs-cli/src/package/git.rs, crates/nvs-cli/src/package/diff.rs, crates/nvs-cli/src/package/index.rs, docs/rules/packaging/the-registry-builds-the-archive-from-git.md, docs/rules/packaging/publishing-never-overwrites.md]
    extra:
      context:
        rules: [packaging/the-registry-builds-the-archive-from-git, packaging/publishing-never-overwrites, packaging/only-a-major-may-break-the-api, packaging/a-package-cannot-reach-its-host, packaging/a-git-dependency-is-root-only, packaging/a-package-is-its-digest]
  - id: 4
    title: a release waits seven days unless an owner confirms it, an owner unlists, and the operator only withholds
    status: open
    context: [crates/nvs-cli/src/package/log.rs, docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md, docs/rules/packaging/an-owner-may-unlist.md, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/a-release-waits-seven-days-unless-an-owner-confirms, packaging/a-known-bad-version-is-retracted-not-deleted, packaging/an-owner-may-unlist, packaging/the-operator-only-withholds, packaging/the-registry-keeps-an-append-only-log]
  - id: 5
    title: an owner is notified by Web Push, webhooks, feeds and an inbox, never by email
    status: open
    context: [crates/nvs-stdlib/src/webpush.rs, crates/nvs-config/src/capability.rs, docs/rules/packaging/an-owner-is-notified-without-email.md, docs/rules/core-classes/web-push-sends-an-encrypted-message.md]
    extra:
      context:
        rules: [packaging/an-owner-is-notified-without-email, core-classes/web-push-sends-an-encrypted-message, packaging/a-release-waits-seven-days-unless-an-owner-confirms]
  - id: 6
    title: an account stays active by proof, never by token, and the service has its guide and proofs
    status: open
    context: [docs/rules/packaging/an-account-stays-active-by-proof-not-by-token.md, docs/reference/tools/35-packages.md]
    extra:
      context:
        rules: [packaging/an-account-stays-active-by-proof-not-by-token, packaging/an-owner-is-notified-without-email, packaging/a-registry-account-is-a-passkey]
checks:
  - id: registry-tests
    stage: 6
    name: the registry application's own tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry]
  - id: service-proofs
    stage: 6
    name: the publishing commands owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/nvs-pkg-login-and-nvs-pkg-whoami", "tools:packages/nvs-pkg-publish", "tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist"]
  - id: owners-closes
    stage: 6
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, registry-service]
    reach: true
    floor: false
extra:
  milestone: M15
---

# The registry is a Novis application that builds every release from git

## Why

The client from goal `package-client` reads a static index that nothing writes yet. This goal builds
the service that writes it: accounts, vendors, tokens, publishing, the pending window, unlisting,
withholding and owner notifications, as a Novis web application in this repository. It is also the
largest program written in Novis, so it tests the language under real use. The byte work (clone, pack,
digest, manifest and API checks, log append) stays in Rust, in the `nvs` subcommands the client
already runs, so a publisher's `nvs pkg publish --dry-run` predicts exactly what the registry builds.

## Standing decisions

**The service is `apps/registry/`, a Novis application like `apps/notes/`.** It stores its data in
SQLite through `Core\Db`, signs users in with `Core\Passkey` (goal `core-passkey`), and runs `nvs`
subcommands as child processes, granted in its own `nvs.toml`. Its tests run with
`nvs test apps/registry` against a temporary database, a push service and a webhook receiver on
loopback listeners, and a git repository served from a loopback listener the test grants.

**No email, no SMS, no password, and no binding to any forge.** There is no GitHub or GitLab sign-in
and no OIDC trusted publishing. An account is one or more passkeys and one-time recovery codes shown
once at sign-up. Sign-up carries a small proof-of-work against bulk sign-ups. An account can do nothing
until its first publish, which needs a public git repository. There is no human recovery: losing every
passkey and every code loses the account. The account's packages keep working, and its co-owners keep
publishing. The guide advises two owners per vendor. Contact with an owner is the inbox on the website
and the package's public repository.

**A vendor is owned.** The first publish of `Acme\Http` takes that name and makes the account the
owner of the vendor `Acme`. Only the vendor's owners publish more `Acme\...` packages, and an owner may
add other owners. Nothing is reserved in advance, nothing expires, and a name that ever had a published
version is never reassigned. `Core\` and `Novis\` are the project account's alone.

**A publish token** is scoped to one vendor or one package, expires after one year at most, starts with
`nvsreg_` so secret scanners find a leaked one, and is stored by the registry only as a hash.
`nvs pkg login` stores it in the data folder under the ownership check the artifact cache uses
(`crates/nvs-cli/src/cache.rs:1793`).

**The registry builds the archive from git.** `nvs pkg publish` sends a repository URL, a commit, a
version and the package's directory within the repository. `nvs pkg publish` may name a subdirectory, and
the default is the repository root. It uploads no file. The service runs `nvs registry build`, which
clones that commit over HTTPS through `gix`, with size and time limits behind the outbound address
table, packs the directory with `nvs pkg pack`'s code, and digests it. The repository URL is bound at a package's first
publish, and changing it needs a passkey.

**A publish is refused** when a file's namespace is outside the package's name, when the archive holds
an `nvs.toml`, when the manifest names a git dependency, when the repository is over the size limit,
when a minor or patch breaks the previous release's API (`nvs pkg diff` from goal `api-compat`),
when the version was published before, when the account is not active, and when the account has no
verified notification channel.

**A release by token alone is pending for seven days.** The period is fixed and has no setting. A
pending version is fetchable at once by its exact version, and `nvs pkg add` and `nvs pkg update` do not
select it. An owner's passkey confirmation on the website releases it at once. A release whose
dependency is still pending waits until that dependency is released. A retraction or an advisory sent
with a token alone waits the same seven days, and with a passkey it is immediate. The first release at
`0.1.0` or higher waits for a one-time passkey confirmation that shows the exact API being frozen.

**An owner may unlist; the operator may only withhold.** Unlisting needs a passkey, hides a package or
a version from search and listings, keeps it fetchable, is logged, and can be reversed. The operator
may withhold a version's bytes for malware or a legal order, always as a signed, logged advisory with
its reason. The operator never deletes, edits or reassigns, and there is no review queue. A short
policy page says so (goal `registry-website` writes the page). Every event that changes ownership,
tokens, the repository binding, listing, retraction, withholding or confirmation is a log entry
through `nvs registry log append` (goal `registry-log`).

**Owners are notified without email** (`rule:packaging/an-owner-is-notified-without-email`). Each
publish, new token, owner change, unlist and retraction reaches every owner through the channels that
owner set up: Web Push, and webhooks. A webhook is a signed POST to a URL the owner gives, sent through
the outbound address table, with retries. Always present, whatever the owner set up: a signed feed per
vendor, the inbox on the website, and `nvs pkg publish` and `nvs pkg whoami` printing the pending releases the
caller did not confirm. `nvs pkg publish` warns when no owner of the vendor has a working channel. A public
feed lists every pending release.

**A verified channel is mandatory.** An account with no verified Web Push device and no verified
webhook cannot publish and cannot be added as an owner. A push device is verified by the action on a
test push, which sends back the one-time code inside the encrypted payload. A webhook is verified by a
one-time code the registry sends to it, which the owner returns on the website or by the link in the
message. A channel that stops working stops counting: the push service reports it gone, or a webhook
still fails after its retries. So an account whose last channel fails cannot publish until it
verifies one again. An owner may register any number of push devices; a device is added
only while signed in with a passkey, and adding or removing one notifies every other device and webhook
of that account. A subscription the push service reports gone is dropped, and the account page lists
each device with the date it last answered. Every push message is sent through `Core\WebPush` (goal
`core-web-push`, `rule:core-classes/web-push-sends-an-encrypted-message`), which encrypts the payload,
signs it with the registry's VAPID key and sends it under the ordinary `net.connect` grant.

**An account must show activity within 60 days, or it can no longer publish**
(`rule:packaging/an-account-stays-active-by-proof-not-by-token`). The 60 days are fixed. Its packages
stay available and its releases are untouched. Activity is a passkey sign-in, or the "Yes, I am here"
answer to a Web Push reminder: the reminder carries a one-time code inside the encrypted payload, and
the action button, or the notification itself through a small page, sends that code back. A webhook
reminder carries a one-time "I am here" link that counts the same way, so a webhook-only owner can
answer too. Token use never counts. An answer keeps an active account active, and cannot revive a lapsed one; after 60 days
without activity, only a passkey sign-in makes the account active again. The owner chooses when the
reminders start, such as after 14, 30 or 45 quiet days.

**Downloads are counted without storing an address.** The service serves archives itself and counts
per version per day. One count per address per day goes through a hash with a daily salt, and the
salt is deleted when the day ends. Copies from mirrors and from `vendor/` are not counted.

**Trade-offs.** No language feature changes, and the request path of other programs is untouched. The
service is a program its operator runs; its memory is per request, with SQLite on disk. Security is
the reason for every choice above: no forge to compromise, no password to phish, no mailbox to take
over, a build the registry does itself, a pending window anyone can watch, an account that a stolen
token cannot keep alive, and an operator who cannot rewrite anything. The cost to a publisher is a
passkey, one verified notification channel, and, for a hotfix, one confirmation on the website.

## Stages

### 1. the application exists, and an account is passkeys, recovery codes and a proof-of-work sign-up

**Does:** a person signs up with a passkey after a small proof-of-work, receives recovery codes once,
adds more passkeys and signs in.

- [ ] `apps/registry/` with its `nvs.toml`, schema and test setup, in the shape of `apps/notes/` (`rule:packaging/the-registry-is-a-novis-program`). `apps/notes/src/App.nvs:1`
- [ ] Sign-up with its proof-of-work, passkeys through `Core\Passkey`, recovery codes stored hashed, and sign-in; nothing asks for an email (`rule:packaging/a-registry-account-is-a-passkey`). `apps/registry/src/Accounts.nvs:1`

### 2. a vendor has owners, and a token is scoped, hashed and expires

**Does:** owners manage a vendor, and a token publishes only inside its scope until it expires.

- [ ] Vendors and owners, an owner added only with a passkey. The check that the new owner has a verified channel lands with stage 5. `apps/registry/src/Vendors.nvs:1`
- [ ] Tokens: scope, expiry, the `nvsreg_` prefix, hashed storage and revocation (`rule:packaging/a-publish-token-is-scoped-and-expires`). `apps/registry/src/Tokens.nvs:1`
- [ ] `nvs pkg login` stores a token in the data folder under the ownership check, and `nvs pkg whoami` prints the account and its scope. `crates/nvs-cli/src/package/login.rs:1`, `crates/nvs-config/src/data.rs:446`

### 3. a publish builds the archive from a git commit, refuses what the rules refuse, and lands in the static index

**Does:** `nvs pkg publish` sends a commit, the service builds and checks the archive, records a release
that is never overwritten, and serves it in the index the client reads.

- [ ] `nvs registry build`: clone, pack, digest, the namespace and manifest checks, and the API diff against the previous release, as JSON for the service (`rule:packaging/the-registry-builds-the-archive-from-git`). `crates/nvs-cli/src/package/build.rs:1`
- [ ] `nvs pkg publish`, with `--dry-run` running the same packing locally to print the digest. `crates/nvs-cli/src/package/publish.rs:1`
- [ ] The service's publish endpoint, the repository binding, and every refusal in the standing decision (`rule:packaging/publishing-never-overwrites`). `apps/registry/src/Publish.nvs:1`
- [ ] The static index in the client's schema, archive downloads, and the salted daily count. `apps/registry/src/Index.nvs:1`, `crates/nvs-cli/src/package/index.rs:1`

### 4. a release waits seven days unless an owner confirms it, an owner unlists, and the operator only withholds

**Does:** a token-only release is pending for seven days, an owner's passkey releases it or freezes
the first stable API at once, and unlisting and withholding work as their rules say, each logged.

- [ ] The pending state, the dependency wait, the passkey confirmation and the one-time stable confirmation (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). `apps/registry/src/Releases.nvs:1`
- [ ] `nvs pkg retract`, with the same wait for a token and none for a passkey (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `crates/nvs-cli/src/package/retract.rs:1`
- [ ] `nvs pkg unlist` and its passkey step, reversible (`rule:packaging/an-owner-may-unlist`), and the operator's withhold as a signed advisory with its reason (`rule:packaging/the-operator-only-withholds`). `crates/nvs-cli/src/package/unlist.rs:1`, `apps/registry/src/Withhold.nvs:1`
- [ ] A log entry for every event in the standing decision. `apps/registry/src/Events.nvs:1`, `crates/nvs-cli/src/package/log.rs:1`

### 5. an owner is notified by Web Push, webhooks, feeds and an inbox, never by email

**Does:** every event reaches each owner through the channels they set up and through the channels
that are always present, and an account without a verified channel cannot publish or become an owner.

- [ ] Web Push: device registration while signed in with a passkey, each message sent through `Core\WebPush`, a gone subscription dropped, and each device's last answer shown (`rule:packaging/an-owner-is-notified-without-email`, `rule:core-classes/web-push-sends-an-encrypted-message`). `apps/registry/src/Push.nvs:1`, `crates/nvs-stdlib/src/webpush.rs:1`
- [ ] Webhooks: a signed POST through the outbound address table, with retries. `apps/registry/src/Webhooks.nvs:1`, `crates/nvs-config/src/capability.rs:262`
- [ ] Verification: a test push answered with its one-time code, a webhook code returned on the website or by its link, and a channel that stops working no longer counting. `apps/registry/src/Channels.nvs:1`
- [ ] A publish, and adding an owner, refused for an account with no verified channel. `apps/registry/src/Publish.nvs:1`, `apps/registry/src/Vendors.nvs:1`
- [ ] The signed feed per vendor, the website inbox, the public pending feed, and a device added or removed notifying every other channel of the account. `apps/registry/src/Feeds.nvs:1`
- [ ] `nvs pkg publish` and `nvs pkg whoami` print the pending releases the caller did not confirm, and `nvs pkg publish` warns when no owner of the vendor has a working channel. `crates/nvs-cli/src/package/publish.rs:1`

### 6. an account stays active by proof, never by token, and the service has its guide and proofs

**Does:** an account with no passkey sign-in or reminder answer for 60 days can no longer publish, and
the service can be deployed from its guide.

- [ ] The activity clock: a passkey sign-in or a reminder answer counts, token use never does, and after 60 days only a passkey sign-in revives the account (`rule:packaging/an-account-stays-active-by-proof-not-by-token`). `apps/registry/src/Activity.nvs:1`
- [ ] The "Yes, I am here" reminder: the start the owner chose, the one-time code in the encrypted push payload, the small page the notification opens, and the one-time link in a webhook message. `apps/registry/src/Push.nvs:1`, `apps/registry/src/Webhooks.nvs:1`
- [ ] The deploy guide in `apps/registry/README.md`, and the client's tests run against the service. `apps/registry/README.md:1`
- [ ] The headings `# nvs pkg login and nvs pkg whoami`, `# nvs pkg publish` and `# nvs pkg retract and nvs pkg unlist` in `docs/reference/tools/35-packages.md`, each owing nothing; every rule this goal owns `shipped`; `bun nv owners --closes registry-service` passes. `docs/reference/tools/35-packages.md:1`

## Handoff

### State

The goal is new. No stage is started. It needs goals `api-compat`, `package-client`, `registry-log`,
`core-passkey` and `core-web-push` reached.

### Next

Stage 1, the application and its accounts:
- `apps/registry/` in the shape of `apps/notes/` (`rule:packaging/the-registry-is-a-novis-program`). `apps/notes/src/App.nvs:1`
- sign-up with its proof-of-work, and sign-in with `Core\Passkey` (`rule:packaging/a-registry-account-is-a-passkey`). `apps/registry/src/Accounts.nvs:1`

### Backlog

- Search, package pages, account pages and the policy page are goal `registry-website`'s (`docs/goals/registry-website.md`).
- The advisory feed and `nvs pkg audit` are goal `package-audit`'s (`docs/goals/package-audit.md`).
