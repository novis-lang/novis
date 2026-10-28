---
looper: 1
title: The registry is a Novis application that builds every release from git
summary: "apps/registry is a Novis web application with passkey accounts, recovery codes and a proof-of-work sign-up, owned vendors, scoped expiring tokens, publishing that builds the archive from a git commit and refuses what its rules refuse, a seven-day pending window an owner's passkey ends early, unlisting, withholding, a log entry per event, owner notifications by Web Push, webhooks, feeds and an inbox with no email and a verified channel required to publish or become an owner, a 60-day activity rule that token use never satisfies, the signed static index written to the stores one settings file names with every publish durable before it is visible, a sealed database backup and a restore on start, downloads counted from the file hosts' access lines; nvs pkg login, whoami, publish, retract and unlist work against it, it has a deploy guide, the commands have their feature proofs, every designed rule this goal owns is shipped, and the checks below pass."
context: [apps/registry/README.md, apps/registry/src/App.nvs, crates/nvs-cli/src/package/mod.rs, docs/rules/packaging/the-registry-is-a-novis-program.md]
stages:
  - id: 1
    title: the application exists, and an account is passkeys, recovery codes and a proof-of-work sign-up
    status: done
    context: [apps/notes/src/App.nvs, apps/notes/README.md, crates/nvs-stdlib/src/passkey.rs, docs/rules/packaging/a-registry-account-is-a-passkey.md]
    extra:
      context:
        rules: [packaging/the-registry-is-a-novis-program, packaging/a-registry-account-is-a-passkey, core-classes/passkey-verifies-a-webauthn-ceremony]
  - id: 2
    title: a vendor has owners, and a token is scoped, hashed and expires
    status: done
    context: [crates/nvs-config/src/data.rs, crates/nvs-cli/src/cache.rs, docs/rules/packaging/a-publish-token-is-scoped-and-expires.md, docs/rules/packaging/a-package-name-is-vendor-slash-name.md]
    extra:
      context:
        rules: [packaging/a-publish-token-is-scoped-and-expires, packaging/a-package-name-is-vendor-slash-name, config/ownership-is-the-trust-boundary]
  - id: 3
    title: "`nvs registry build` builds the archive from a git commit, and `nvs pkg publish` sends one"
    status: done
    context: [crates/nvs-cli/src/package/pack.rs, crates/nvs-cli/src/package/git.rs, crates/nvs-cli/src/package/diff.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/Cargo.toml, docs/rules/packaging/the-registry-builds-the-archive-from-git.md]
    extra:
      context:
        rules: [packaging/the-registry-builds-the-archive-from-git, packaging/only-a-major-may-break-the-api, packaging/a-package-cannot-reach-its-host, packaging/a-git-dependency-is-root-only, packaging/a-package-is-its-digest]
  - id: 7
    title: the service's publish refuses what the rules refuse, and the release lands in the static index
    status: done
    context: [apps/registry/src/App.nvs, apps/registry/src/Vendors.nvs, crates/nvs-cli/src/package/index.rs, docs/rules/packaging/publishing-never-overwrites.md, docs/rules/packaging/a-package-is-its-digest.md]
    extra:
      context:
        rules: [packaging/publishing-never-overwrites, packaging/the-registry-builds-the-archive-from-git, packaging/a-package-is-its-digest, packaging/a-package-name-is-vendor-slash-name]
  - id: 11
    title: the hosting is one settings file, a publish is durable before it is visible, and the service backs itself up and restores itself
    status: open
    context: [apps/registry/hosting.json, apps/registry/src/Site.nvs, apps/registry/src/Stores.nvs, apps/registry/src/Index.nvs, docs/rules/packaging/registry-hosting-is-configuration.md]
    extra:
      context:
        rules: [packaging/registry-hosting-is-configuration, packaging/search-sorts-by-the-readers-choice, packaging/the-registry-keeps-an-append-only-log]
  - id: 4
    title: a release waits seven days unless an owner confirms it, an owner unlists, and the operator only withholds
    status: open
    context: [crates/nvs-cli/src/package/log.rs, docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md, docs/rules/packaging/an-owner-may-unlist.md, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/a-release-waits-seven-days-unless-an-owner-confirms, packaging/a-known-bad-version-is-retracted-not-deleted, packaging/an-owner-may-unlist, packaging/the-operator-only-withholds, packaging/the-registry-keeps-an-append-only-log]
  - id: 5
    title: an owner is notified by Web Push, a channel is verified, and an account without one cannot publish
    status: open
    context: [crates/nvs-stdlib/src/webpush.rs, apps/registry/src/Vendors.nvs, docs/rules/packaging/an-owner-is-notified-without-email.md, docs/rules/core-classes/web-push-sends-an-encrypted-message.md]
    extra:
      context:
        rules: [packaging/an-owner-is-notified-without-email, core-classes/web-push-sends-an-encrypted-message]
  - id: 8
    title: an owner is notified by a signed webhook with retries, verified by a one-time code
    status: open
    context: [crates/nvs-config/src/capability.rs, apps/registry/src/Channels.nvs, docs/rules/packaging/an-owner-is-notified-without-email.md]
    extra:
      context:
        rules: [packaging/an-owner-is-notified-without-email, packaging/a-package-cannot-reach-its-host]
  - id: 9
    title: the feeds, the inbox and the commands show every event, never by email
    status: open
    context: [apps/registry/src/Push.nvs, apps/registry/src/Webhooks.nvs, crates/nvs-cli/src/package/publish.rs, crates/nvs-cli/src/package/login.rs, docs/rules/packaging/an-owner-is-notified-without-email.md]
    extra:
      context:
        rules: [packaging/an-owner-is-notified-without-email, packaging/a-release-waits-seven-days-unless-an-owner-confirms]
  - id: 6
    title: an account stays active by proof, never by token
    status: open
    context: [docs/rules/packaging/an-account-stays-active-by-proof-not-by-token.md, apps/registry/src/Push.nvs, apps/registry/src/Webhooks.nvs]
    extra:
      context:
        rules: [packaging/an-account-stays-active-by-proof-not-by-token, packaging/an-owner-is-notified-without-email, packaging/a-registry-account-is-a-passkey]
  - id: 10
    title: the service has its deploy guide and its proofs, and its rules are shipped
    status: open
    context: [apps/registry/README.md, docs/reference/tools/35-packages.md]
    extra:
      context:
        rules: [packaging/the-registry-is-a-novis-program]
  - id: 12
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: registry-tests
    stage: 10
    name: the registry application's own tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry, --config, apps/registry/nvs.toml]
  - id: service-proofs
    stage: 10
    name: the publishing commands owe no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/nvs-pkg-login-and-nvs-pkg-whoami", "tools:packages/nvs-pkg-publish", "tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist"]
  - id: owners-closes
    stage: 10
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, registry-service]
    reach: true
    floor: false
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
`nvs test apps/registry --config apps/registry/nvs.toml` against the app's own database file and a
push service and a webhook receiver on loopback listeners. The app runs `nvs registry build` through
one class, and its tests replace that class with `Core\Test::double` fed the JSON that
`crates/nvs-cli/tests/registry_build.rs` pins.

**The tests share `registry.sqlite`, and each test makes its own accounts.** `Core\Test::request`
runs the app in a fresh run, so neither `#[Test(db:)]`'s rollback nor a `:memory:` database reaches
the rows a request writes (probed: the count grew across runs). Every test signs up new accounts with
random ids through `Registry\BrowserTest` and reads only those, so tests run in parallel and in any
order. `registry.sqlite` is gitignored. Why: the tested path is the real request path, with no test
switch in the app. A later need for a clean file is one `--config` naming another path.

**The API is JSON, and the gate starts the session.** Every route takes and returns JSON; the HTML
pages are goal `registry-website`'s. `Registry\Gate::allows` calls `Core\Session::start()` once per
request, because a second `start()` throws, and `Visitor::SignedIn` routes need the session's account.
Account ids are 32 hex characters, checked by `Accounts::idOf` wherever one is read back from the
session or a row. `hosting.json` holds `origin` (the passkey domain is its host) and `signUpWork`, the
proof-of-work's zero bits: 8 in the committed file so a test solves one in milliseconds, and the
deploy guide (stage 10) gives a production figure near 20.

**The test git server speaks HTTPS, never plain HTTP, and only the Rust tests run it.**
`crates/nvs-cli/tests/registry_build.rs` serves `git http-backend` behind a rustls listener with a
certificate `rcgen` issues per run; both are already `nvs-cli` dev-dependencies for a loopback TLS
peer (`crates/nvs-cli/Cargo.toml:131`). `nvs registry build` takes `--ca-file <pem>`, an operator flag
the service passes from its own configuration and never from a publisher, and a publisher's `http:` or
`file:` URL is refused. Why: the tested path is the production path, no insecure switch exists to be
left on, and one git server in one language is simpler than two.

**The clock is Core's, so tests move it.** Every time the service stores or compares (the seven-day
window, the 60-day activity rule, the daily salt, token expiry) comes from Core's clock, never from
SQLite's `now`. Tests fix it with `#[Test(at: ...)]` and move it with `Core\Test::advance`
(`rule:testing/determinism-declared-on-the-test`). Why: a window test then takes no real time and
never flakes at midnight.

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
(`crates/nvs-cli/src/cache.rs:1258`).

**Adding or removing an owner, and making a token, need a passkey sign-in in the last 15 minutes**
(`Session::PASSKEY_WINDOW`, `apps/registry/src/Session.nvs`). A recovery-code sign-in can revoke a
token and add a passkey, and nothing else that needs a passkey. A token is read only from the
`authorization` header, and `Tokens::allows` also checks that its account still owns the vendor, so
removing an owner stops that owner's tokens at once. `Vendors::claim` is what stage 7's first
publish calls. The tests call it directly, because no route takes a vendor without a publish.
Why: a stolen session cookie that is older than 15 minutes, or a leaked recovery code, cannot mint a
publishing key or take over a vendor.

**A token may name a vendor nobody owns yet, and the first publish takes it.** `Vendors::mayPublish`
is true for an owner or an unowned vendor, and both `createToken` and `Tokens::allows` use it;
`Publish` calls `Vendors::claim` only after every other check passed, so a refused publish claims
nothing. Two accounts may hold tokens for one unowned vendor, and the first publish wins. Why: an
account can do nothing before its first publish, so a token that needs an owned vendor could never
be made. Nothing is reserved in advance, as *A vendor is owned* says.

**The build runs only where the operator grants it.** `hosting.json`'s `nvs` names the program, and
the committed `nvs.toml` grants no `[capabilities.process]`: a bare `nvs` that is not on `PATH` is a
configuration error, which would stop every test on a runner without it. The tests double the
`Builder`, and the deploy guide (stage 10) gives the grant with the full path. A package bound to a
repository and folder passes its newest lower release's `api` file as `--previous`; a folder that
builds another name than the one it published is refused. Why: the tests stay independent of the
runner's `PATH`, and the operator states every program the service starts.

**The registry builds the archive from git.** `nvs pkg publish` sends a repository URL, a commit, a
version and the package's directory within the repository. `nvs pkg publish` may name a subdirectory, and
the default is the repository root. It uploads no file. The service runs `nvs registry build`, which
clones that commit over HTTPS through `gix`, with size and time limits behind the outbound address
table, packs the directory with `nvs pkg pack`'s code, and digests it. The repository URL is bound at a package's first
publish, and changing it needs a passkey.

**The build and the publish speak JSON, and the build checks what one commit can tell.**
`nvs registry build <url> <commit>` takes `--dir`, `--version`, `--previous <api.json>` with
`--previous-version`, `--archive <file>`, `--api <file>` and `--ca-file`, and prints
`{"schema":1,"ok":true,"name","version","repository","commit","directory","digest":"blake3:…","size","files":[{"path","size"}],"dependencies":{name:minimum},"capabilities":{"required","optional"},"api"}`
or `{"schema":1,"ok":false,"refusal","api"}` with exit 1 (`crates/nvs-cli/src/package/build.rs`).
It refuses a URL that is not `https:`, a missing name or version, a version other than `--version`,
a git dependency, a declaration outside the namespace (compared without case, over the API
document read from the unpacked archive), every refusal of the pack, and a break the step from
`--previous-version` does not allow. The service stores each release's `--api` file and passes it as
the next build's `--previous`. Everything that needs the database (the version published before, an
active account, a verified channel, the vendor's owner) is the service's check, in stage 7. The
loopback exception for the tests is the working directory's `[capabilities.net] internal`, as for
every package command. `nvs pkg publish` sends `POST /api/publish` with
`{"repository","commit","version","directory"}` and the stored token, and reads `{"digest","message"}`
on a `2xx`, where `message` is the sentence it prints (pending until a date, or released), and
`{"error"}` on a refusal. It packs `HEAD` from a `file:` clone of its own repository with the build's
code first, so its digest is the registry's and uncommitted changes are never in it. Why: one JSON
contract per side keeps the app's `Core\Test::double` and the Rust tests pinned to the same shape.

**The index catches up per package, under the database's write lock.** After `Publish` saves a
release, `App::publish` calls `Index::catchUp($key)` (`apps/registry/src/Index.nvs`), which runs
in one transaction whose first statement writes, so two publishes append to the log one at a
time. It copies each unlogged release's archive to `<work>/archive/<hex>.tar`, appends one
`publish` entry per release through `Log` (`NvsLog` runs `nvs registry log append`, which prints
the files it wrote), stores each release's record number in `releases.log`, then writes
`index/<vendor>/<name>.json` and `registry.json` and signs every file through `nvs registry index
sign --at` with Core's clock. The work folder (`hosting.json`'s `work`) is never served, so an
unsigned file is never visible. The publish is answered `201` only after all of that; a failure
answers `500` and the saved release stays unlogged until the next catch-up of its package (stage
11's daily task adds a sweep of every package). The tests double `Log` as they double `Builder`.
`Index::document` is pure, and `apps/registry/fixtures/index.json` pins its output:
`IndexTest.nvs` compares it byte for byte, and `crates/nvs-cli/src/package/index.rs`'s
`the_registry_services_index_file_reads_back` signs it and reads it with the client's reader. With
no hosts, `App`'s `GET /registry.json`, `/index/…`, `/log/…` and `/archive/…` routes send the files
from the first `dir` store (`apps/registry/src/Files.nvs`), and an archive fetched with a `nvs/`
user agent is counted by `Downloads`. Why: the log is the one serial resource, the database already
serializes writers, and a per-package catch-up keeps one stuck release from blocking another
package. Memory: one archive copy per publish, on disk.

**Every new file waits in `unsent` until every store has it, and the send runs under the same
write lock.** `Index::catchUp` queues each archive, each tile `log append` printed, the index file,
`registry.json` and `log/checkpoint` with a rank (`Stores::ARCHIVE` … `Stores::CHECKPOINT`), and
`Stores::send` (`apps/registry/src/Stores.nvs`) puts every file of one rank to every store before
any file of the next, then deletes the rows. Only the published package's index file is queued; the
other index files are re-signed in the work folder and reach the stores in the daily task. A failed
store returns `false` after the log numbers are committed, so nothing is appended twice, and the
same commit published again answers `200` (`Publish::again`) and sends what is left. Why: two
publishes that sent at once could put an older checkpoint after a newer one, and a file lock cannot
wait under a test's virtual clock, where `Core\Time::sleep` returns at once. Cost: another request
that writes waits while a publish's files reach the stores, a few seconds per publish, which is rare.
A store that fails at the checkpoint rank can leave an earlier store with the new checkpoint; the
retry makes them agree.

**A publish is refused** when a file's namespace is outside the package's name, when the archive holds
an `nvs.toml`, when the manifest names a git dependency, when the clone is over 512 MiB, when the
archive breaks `rule:packaging/a-package-carries-only-its-source-and-what-it-lists` (over 16 MiB or
5,000 files, no license file, a path another platform cannot hold, or a `files` entry that is missing,
a `.nvs` file or already carried), when a `[[download]]` entry is malformed
(`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`; the registry never downloads one),
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

**The service is built and tested on one machine, and hosting is configuration**
(`rule:packaging/registry-hosting-is-configuration`). The committed `apps/registry/hosting.json` names
directories as every store, so `nvs serve apps/registry` and `nvs test apps/registry` need nothing
else. A deployment writes its own `hosting.json`, and nothing in the code names a host or a provider.
The tests run the `http` store, the backup and the access-line source against loopback listeners, as
the push and webhook tests do. Why: the user's call is that the service is finished locally and a
deployment only changes configuration.

**A publish is durable before it is visible.** The archive, the log tiles, the index files signed by
`nvs registry index sign` (goal `registry-log`), and the checkpoint last, go to every public store, and
the publish is answered only after every store confirmed. A daily task signs every index file again.
The service never purges a CDN: the deploy guide gives the files that change a 60-second cache
lifetime.

**The service backs itself up and restores itself.** After each publish, and every five minutes when
something changed, the database is copied as JSON lines of every row (Core's SQLite driver refuses
`VACUUM INTO`), sealed with `Core\Crypto::seal`, and written to the backup store as one file each
copy replaces (`apps/registry/src/Backup.nvs`). A service that starts with no database restores the
newest copy, takes the log from a public store, verifies it with `nvs registry log verify --key
--entries`, and rebuilds every release's state from the log and the index files
(`apps/registry/src/Restore.nvs`, run from `Schema::open` when `releases` is missing). It refuses to
start when the stores have a log and the backup store has no copy, so it never starts a second log.

**Downloads are counted from the file hosts' access lines** (`rule:packaging/search-sorts-by-the-readers-choice`).
A daily task reads the previous day's lines from the source the settings name, counts per version per
day only requests whose user agent is `nvs`, one per address per day through a hash with a daily salt,
and keeps no line. The service counts through the same function when it serves a file itself, as it
does locally. The salt is deleted when the day ends. Copies from mirrors and from `vendor/` are not
counted. Bunny's line format is the first parser.

**Lanes.** The Rust lane (`crates/nvs-cli/src/package/login.rs`, `build.rs`, `publish.rs`,
`retract.rs`, `unlist.rs`) is disjoint from `apps/registry/`, so stage 3 can run beside stage 2's app
items, and stage 4's and stage 9's Rust items can go to their own sessions. Its one shared file is
`crates/nvs-cli/src/main.rs`'s `Command` enum (`crates/nvs-cli/src/main.rs:373`), which goals
`api-compat`, `package-client` and `registry-log` also edit, so those merges are serial. Every app
stage edits `apps/registry/src/App.nvs`'s route wiring and the schema, so no two app stages run at
once. Their order is 1, 2, 7, 11, 4, 5, 8, 9, 6, except that stage 8 needs only stage 2 and may go
earlier. Stages 5 and 8 write disjoint channel files (`Push.nvs`, `Webhooks.nvs`) and share only
`Channels.nvs`. Stage 10 writes `docs/reference/tools/35-packages.md`, where every package
goal adds a heading, so it merges serially with them.

**Gates.** Stage 1 needs `Core\Passkey`: `data/rules/core-classes/passkey-verifies-a-webauthn-ceremony.json`
reads `"status": "shipped"`; until then, the session takes stage 1's first item alone. Stage 2's `nvs pkg
login` needs `crates/nvs-cli/src/package/mod.rs`. Stage 3 needs goals `package-client` and `api-compat`
reached: `docs/goals/package-client.md` and `docs/goals/api-compat.md` are gone. Stages 4 and 11, and
stage 7's signing, need goal `registry-log` reached: `docs/goals/registry-log.md` is gone. Stage 5
needs `Core\WebPush`:
`data/rules/core-classes/web-push-sends-an-encrypted-message.json` reads `"status": "shipped"`; until
then, the session takes stage 8, which needs only stage 2. A session whose stage is gated takes the
earliest open stage whose gate holds, and ends `BLOCKED` naming the missing file when none does.

**Trade-offs.** No language feature changes, and the request path of other programs is untouched. The
service is a program its operator runs; its memory is per request, with SQLite on disk. Security is
the reason for every choice above: no forge to compromise, no password to phish, no mailbox to take
over, a build the registry does itself, a pending window anyone can watch, an account that a stolen
token cannot keep alive, and an operator who cannot rewrite anything. The cost to a publisher is a
passkey, one verified notification channel, and, for a hotfix, one confirmation on the website.
Hosting: downloads keep working while the service is down, because the files are on the file hosts
and the client keeps the last root file. A publish takes one write per store per file, which is slow
next to a local write and rare. Memory: one archive per publish while it is written, and a temporary
database copy the size of the database during a backup. A write outage, or a lost backup interval,
delays publishers and loses at most the rows the log does not hold.

## Stages

### 1. the application exists, and an account is passkeys, recovery codes and a proof-of-work sign-up

**Does:** a person signs up with a passkey after a small proof-of-work, receives recovery codes once,
adds more passkeys and signs in.

**Split:** an `Explore` subagent returns `Core\Passkey`'s calls for registration and sign-in from its
reference card, one line each, and the list of `apps/notes/` files with what each holds.

- [x] `apps/registry/` with its `nvs.toml`, schema and test setup, in the shape of `apps/notes/` (`rule:packaging/the-registry-is-a-novis-program`). `apps/notes/src/App.nvs:1`, `apps/registry/src/App.nvs:1`
- [x] Sign-up with its proof-of-work, passkeys through `Core\Passkey`, recovery codes stored hashed, and sign-in; nothing asks for an email (`rule:packaging/a-registry-account-is-a-passkey`). `apps/registry/src/Accounts.nvs:1`
- [x] Tests: a sign-up without the proof-of-work refused, recovery codes shown once and each usable once, a second passkey added only while signed in. `apps/registry/src/AccountsTest.nvs:1`
- [x] `rule:packaging/a-registry-account-is-a-passkey` is `shipped` with these tests in `guardedBy`. `data/rules/packaging/a-registry-account-is-a-passkey.json:1`

### 2. a vendor has owners, and a token is scoped, hashed and expires

**Does:** owners manage a vendor, and a token publishes only inside its scope until it expires.

**Split:** after the session lands the schema for vendors and tokens, one writing subagent writes
`Tokens.nvs` and its test, then a second writes `login.rs` (the Rust lane).

- [x] Vendors and owners, an owner added only with a passkey. The check that the new owner has a verified channel lands with stage 5. `apps/registry/src/Vendors.nvs:1`
- [x] Tokens: scope, expiry, the `nvsreg_` prefix, hashed storage and revocation (`rule:packaging/a-publish-token-is-scoped-and-expires`). `apps/registry/src/Tokens.nvs:1`
- [x] Tests: a token outside its scope refused, an expired one refused, a revoked one refused, and only its hash stored. `apps/registry/src/TokensTest.nvs:1`
- [x] `nvs pkg login` stores a token in the data folder under the ownership check, and `nvs pkg whoami` prints the account and its scope, with a test of a token file writable by another account. `crates/nvs-cli/src/package/login.rs:1`, `crates/nvs-config/src/data.rs:446`
- [x] `rule:packaging/a-publish-token-is-scoped-and-expires` is `shipped`. `data/rules/packaging/a-publish-token-is-scoped-and-expires.json:1`

### 3. `nvs registry build` builds the archive from a git commit, and `nvs pkg publish` sends one

**Does:** `nvs registry build` clones a commit over HTTPS, packs, digests and checks it and prints JSON
for the service; `nvs pkg publish` sends a commit, and `--dry-run` predicts the digest.

**Split:** an `Explore` subagent lists the functions `pack.rs`, `git.rs` and `diff.rs` export, one
line each with its anchor. After the session lands `build.rs`, a writing subagent writes the HTTPS
loopback git server and the tests.

- [x] `nvs registry build`: clone, pack, digest, the namespace and manifest checks, and the API diff against the previous release, as JSON for the service, with `--ca-file` (`rule:packaging/the-registry-builds-the-archive-from-git`). `crates/nvs-cli/src/package/build.rs:1`, `crates/nvs-cli/src/main.rs:373`
- [x] `nvs pkg publish`, with `--dry-run` running the same packing locally to print every file, its size, the total and the digest, and both downloading every `[[download]]` entry and refusing one whose bytes do not match. `crates/nvs-cli/src/package/publish.rs:1`
- [x] Tests against the HTTPS loopback server: the JSON a build prints, the dry-run digest equal to the build's, an `http:` URL refused, a clone over 512 MiB refused, an archive over its limits refused, a malformed `[[download]]` entry refused, a clone that fetches the commit and none of its history, and a namespace outside the name refused. `crates/nvs-cli/tests/registry_build.rs:1`

### 7. the service's publish refuses what the rules refuse, and the release lands in the static index

**Does:** the service runs the build, records a release that is never overwritten, and serves it in
the index the client reads.

**Split:** after the session lands `Publish.nvs`, one writing subagent writes the refusal tests, then
a second writes `Index.nvs`'s download count and its test.

- [x] The service's publish endpoint, the build class the tests double, the repository binding, and every refusal in the standing decision (`rule:packaging/publishing-never-overwrites`). `apps/registry/src/Publish.nvs:1`
- [x] Tests: one per refusal in the standing decision, a republished version refused, and a changed repository URL refused without a passkey. `apps/registry/src/PublishTest.nvs:1`
- [x] The static index in the client's schema, signed with `nvs registry index sign`, the root file with the file hosts, and the salted daily count as the service serves a file itself. `apps/registry/src/Index.nvs:1`, `crates/nvs-cli/src/package/index.rs:1`
- [x] Tests: the client's index reader parses and verifies what the service writes, and one address counts once per day across a day change. `apps/registry/src/IndexTest.nvs:1`
- [x] `rule:packaging/publishing-never-overwrites` and `rule:packaging/the-registry-builds-the-archive-from-git` are `shipped`. `data/rules/packaging/publishing-never-overwrites.json:1`, `data/rules/packaging/the-registry-builds-the-archive-from-git.json:1`

### 11. the hosting is one settings file, a publish is durable before it is visible, and the service backs itself up and restores itself

**Does:** the service reads its stores, file hosts, backup and download-log source from
`hosting.json`, writes each publish to every store before it answers, backs up a sealed copy of its
database, restores itself on a start with no database, and counts downloads from access lines.

**Split:** after the session lands `Stores.nvs` and the settings reader, one writing subagent writes
`Backup.nvs` and its test, then a second writes `Downloads.nvs` and its test.

- [x] `hosting.json` and its reader: `dir` and `http` stores, the file hosts, the backup store and the download-log source, each secret a path read through `Core\Secret`, and a missing path stopping the start (`rule:packaging/registry-hosting-is-configuration`). `apps/registry/hosting.json:1`, `apps/registry/src/Stores.nvs:1`
- [x] The publish writes the archive, the tiles, the index files and the checkpoint last to every public store, and fails when one store fails; the daily task signs every index file again. `apps/registry/src/Stores.nvs:1`, `apps/registry/src/Publish.nvs:1`
  - [x] `nvs registry log append` prints each file it wrote, `checkpoint` last (`crates/nvs-cli/src/registry.rs`).
  - [x] `Index::catchUp` queues every new file in `unsent`, and `Stores::send` sends them rank by rank under the write lock.
  - [x] A failed store fails the publish, and the same commit published again answers `200` and sends what is left.
  - [x] The daily task: `nvs registry index sign` over every file, every index file queued and sent, and every package with an unlogged release caught up.
- [x] The sealed database copy after each publish and every five minutes when something changed, and the restore on a start with no database, which rebuilds release state from the verified log. `apps/registry/src/Backup.nvs:1`
- [x] The daily count from Bunny's access lines through the counting function, keeping no line. `apps/registry/src/Downloads.nvs:1`
- [ ] Tests against loopback listeners: an `http` store receiving each file in order, a publish failed by a store that answers `500` with no checkpoint moved, a backup the store cannot open without the key, a restore that brings back every release's state, and access lines that count one address once and skip a user agent that is not `nvs`. `apps/registry/src/StoresTest.nvs:1`, `apps/registry/src/BackupTest.nvs:1`, `apps/registry/src/DownloadsTest.nvs:1`
  - [x] A sealed copy opens only with its key, and a restore brings back every release (`BackupTest.nvs`).
  - [x] Access lines count one address once and skip a user agent that is not `nvs` (`DownloadsTest.nvs`).
  - [ ] An `http` store receives each file in order, and a store that answers `500` fails the publish with no checkpoint moved (`StoresTest.nvs` or a Rust test with a loopback listener).
- [ ] `rule:packaging/registry-hosting-is-configuration` is `shipped`. `data/rules/packaging/registry-hosting-is-configuration.json:1`

### 4. a release waits seven days unless an owner confirms it, an owner unlists, and the operator only withholds

**Does:** a token-only release is pending for seven days, an owner's passkey releases it or freezes
the first stable API at once, and unlisting and withholding work as their rules say, each logged.

**Split:** after the session lands `Releases.nvs` and `Events.nvs`, one writing subagent writes
`retract.rs` and `unlist.rs` (the Rust lane), then a second writes the tests.

- [ ] The pending state, the dependency wait, the passkey confirmation and the one-time stable confirmation (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). `apps/registry/src/Releases.nvs:1`
- [ ] `nvs pkg retract`, with the same wait for a token and none for a passkey (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). `crates/nvs-cli/src/package/retract.rs:1`
- [ ] `nvs pkg unlist` and its passkey step, reversible (`rule:packaging/an-owner-may-unlist`), and the operator's withhold as a signed advisory with its reason (`rule:packaging/the-operator-only-withholds`). `crates/nvs-cli/src/package/unlist.rs:1`, `apps/registry/src/Withhold.nvs:1`
- [ ] A log entry through `nvs registry log append` for every event in the standing decision. `apps/registry/src/Events.nvs:1`, `crates/nvs-cli/src/package/log.rs:1`
- [ ] Tests under a fixed clock: a token release selectable after seven days and not before, a passkey release at once, a release behind a pending dependency, an unlist reversed, a withhold carrying its reason, and one log entry per event. `apps/registry/src/ReleasesTest.nvs:1`
- [ ] `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`, `rule:packaging/an-owner-may-unlist` and `rule:packaging/the-operator-only-withholds` are `shipped`. `data/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.json:1`, `data/rules/packaging/an-owner-may-unlist.json:1`, `data/rules/packaging/the-operator-only-withholds.json:1`

### 5. an owner is notified by Web Push, a channel is verified, and an account without one cannot publish

**Does:** an owner registers push devices, a device is verified by its test push, and an account with
no verified channel cannot publish or become an owner.

**Split:** after the session lands `Channels.nvs`'s verified state, one writing subagent writes
`Push.nvs`, then a second writes the tests.

- [ ] Web Push: device registration while signed in with a passkey, each message sent through `Core\WebPush`, a gone subscription dropped, and each device's last answer shown (`rule:packaging/an-owner-is-notified-without-email`, `rule:core-classes/web-push-sends-an-encrypted-message`). `apps/registry/src/Push.nvs:1`, `crates/nvs-stdlib/src/webpush.rs:1`
- [ ] Verification: a test push answered with its one-time code, and a channel that stops working no longer counting. `apps/registry/src/Channels.nvs:1`
- [ ] A publish, and adding an owner, refused for an account with no verified channel. `apps/registry/src/Publish.nvs:1`, `apps/registry/src/Vendors.nvs:1`
- [ ] Tests against the loopback push service: a device verified by its code, a gone device dropped, and both refusals. `apps/registry/src/PushTest.nvs:1`

### 8. an owner is notified by a signed webhook with retries, verified by a one-time code

**Does:** an owner gives a webhook URL, the registry verifies it with a one-time code and then sends
every event to it as a signed POST, with retries.

**Split:** after the session lands the signed POST, a writing subagent writes the tests.

- [ ] Webhooks: a signed POST through the outbound address table, with retries. `apps/registry/src/Webhooks.nvs:1`, `crates/nvs-config/src/capability.rs:262`
- [ ] Verification: a one-time code returned on the website or by the link in the message, and a webhook that still fails after its retries no longer counting. `apps/registry/src/Channels.nvs:1`
- [ ] Tests against the loopback receiver: the signature checks, a retry after a failure, an internal address refused, and a failing webhook that stops counting. `apps/registry/src/WebhooksTest.nvs:1`

### 9. the feeds, the inbox and the commands show every event, never by email

**Does:** the channels that are always present carry every event, and the commands print what the
caller did not confirm.

**Split:** `Feeds.nvs` and the Rust lane (`publish.rs`, `login.rs`) are disjoint: one writing subagent
writes the Rust items after the session lands the feeds.

- [ ] The signed feed per vendor, the website inbox, the public pending feed, and a device added or removed notifying every other channel of the account. `apps/registry/src/Feeds.nvs:1`
- [ ] Tests: each event kind in the vendor feed and the inbox, and a device added notifying the other device and the webhook. `apps/registry/src/FeedsTest.nvs:1`
- [ ] `nvs pkg publish` and `nvs pkg whoami` print the pending releases the caller did not confirm, and `nvs pkg publish` warns when no owner of the vendor has a working channel, with tests. `crates/nvs-cli/src/package/publish.rs:1`, `crates/nvs-cli/src/package/login.rs:1`
- [ ] `rule:packaging/an-owner-is-notified-without-email` is `shipped`. `data/rules/packaging/an-owner-is-notified-without-email.json:1`

### 6. an account stays active by proof, never by token

**Does:** an account with no passkey sign-in or reminder answer for 60 days can no longer publish.

- [ ] The activity clock: a passkey sign-in or a reminder answer counts, token use never does, and after 60 days only a passkey sign-in revives the account (`rule:packaging/an-account-stays-active-by-proof-not-by-token`). `apps/registry/src/Activity.nvs:1`
- [ ] The "Yes, I am here" reminder: the start the owner chose, the one-time code in the encrypted push payload, the small page the notification opens, and the one-time link in a webhook message. `apps/registry/src/Push.nvs:1`, `apps/registry/src/Webhooks.nvs:1`
- [ ] Tests under a fixed clock moved by `Core\Test::advance`: token use on day 59 does not count, a reminder answer does, and on day 61 only a passkey sign-in revives. `apps/registry/src/ActivityTest.nvs:1`
- [ ] `rule:packaging/an-account-stays-active-by-proof-not-by-token` is `shipped`. `data/rules/packaging/an-account-stays-active-by-proof-not-by-token.json:1`

### 10. the service has its deploy guide and its proofs, and its rules are shipped

**Does:** the service can be deployed from its guide, the commands are features with their proofs, and
every rule this goal owns is `shipped`.

**Split:** after the session writes the three headings, one writing subagent per heading, one after
another, writes its `about.md` and the examples `bun nv proofs --id` lists, then one writes the three
attacks. All follow AGENTS.md § *Text an end user reads*.

- [ ] The deploy guide in `apps/registry/README.md`, and the client's tests run against the service. It gives a `hosting.json` for one VPS with Bunny Storage in two regions as the public store and a second provider as the backup store, the two cache lifetimes, the registry and the telemetry collector as two accounts in two service units, the log key kept offline and encrypted, and the old host stopped before a new one restores (`rule:packaging/registry-hosting-is-configuration`). `apps/registry/README.md:1`
- [ ] The headings `# nvs pkg login and nvs pkg whoami`, `# nvs pkg publish` and `# nvs pkg retract and nvs pkg unlist`. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and examples for `nvs pkg login and nvs pkg whoami`. `docs/examples/tools/packages/nvs-pkg-login-and-nvs-pkg-whoami/about.md:1`
- [ ] `about.md` and examples for `nvs pkg publish`. `docs/examples/tools/packages/nvs-pkg-publish/about.md:1`
- [ ] `about.md` and examples for `nvs pkg retract and nvs pkg unlist`. `docs/examples/tools/packages/nvs-pkg-retract-and-nvs-pkg-unlist/about.md:1`
- [ ] The attacks: a stolen token publishing outside its scope, a publish of an internal repository URL, and a token-only retraction trying to skip the wait. `tests/hostile/tools/packages/nvs-pkg-publish/01-a-token-outside-its-scope.nvs:1`
- [ ] `rule:packaging/the-registry-is-a-novis-program` is `shipped`, and `bun nv owners --closes registry-service` passes. `data/rules/packaging/the-registry-is-a-novis-program.json:1`

### 12. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 2, 3 and 7 are done. Of stage 11, the settings, the publish to every store, the daily
task, the backup and restore, and the access-line count have landed; the `http` store test and the
rule's `shipped` status are left.

`Index::daily()` runs from `apps/registry/jobs/daily.nvs` (`[[schedule]]`, `scope = "host"`:
`fleet` needs a shared store), then `Downloads::countLog` counts the day before's lines.
`Backup.nvs`, `Restore.nvs` and `jobs/backup.nvs` follow the rewritten standing decision. The
checker now takes two `autoload` lines of one prefix with the same roots as one home
(`crates/nvs-hir/src/autoload.rs`); before it, every folder run of `apps/registry` was E0315.
Run once: `BackupTest` and `DownloadsTest` (2 passed each), and the new autoload unit test.
The sweep's `env-secret-proofs` red is stale (log from 2026-10-22 reads `exit 0`; no check has
that name now), and nightly run 40662443492 was cancelled by a newer push.

Expected red: none known. The conformance case
`tests/conformance/lang/a-duplicate-autoload-prefix-is-a-compile-error.nvst` uses two different
roots and should stay green.

### Next

Stage 11's last test and its rule. `Core\Test` has no loopback listener but `server: true` (the
app's own server), so the `http` store test needs a decision: a Rust test in `crates/nvs-cli/tests/`
that runs a loopback `PUT` listener and a publish against `nvs serve` with a `hosting.json` in a
scratch copy of the app (as `telemetry_e2e.rs` runs an app) is the recommendation, since it adds no
test route to the app. Then set the rule `shipped` and stage 11 `done`.
- [ ] The `http` store test: each file in order, and a `500` store failing the publish with no checkpoint moved. `crates/nvs-cli/tests/telemetry_e2e.rs:1`, `apps/registry/src/Store.nvs:1`
- [ ] `rule:packaging/registry-hosting-is-configuration` is `shipped`, and its *Not on disk* paragraph is removed. `data/rules/packaging/registry-hosting-is-configuration.json:1`

### Backlog

- Bunny's access-line field order in `Downloads::parseBunny` (`apps/registry/src/Downloads.nvs`)
  and the day path `MM-DD-YY/<file>` were written from memory, with no web tool to check them.
  Check both against Bunny's logging documentation before stage 10's deploy guide names them.
  Gzipped day files and a missed day's catch-up are not handled.
- Restore leaves gaps: an index file's signature is not checked during a restore, a rebuilt
  package has an empty `directory`, and the `builds/` API files are not in the copy, so a rebuilt
  release's next build has no `--previous`. Stage 10's guide says so, or the copy takes them.
- `Core` gaps the backup hit: `Db::open` with `Db\Driver::Sqlite` written in full passes `check`
  and panics in `nvs-ir` lowering (`expr.rs:504`), a `readOnly` transaction throws on SQLite, and
  there is no way to read a local file as bytes (the copy is base64 for that reason).
- `StoresTest.nvs` must not share `unsent` with the parallel tests: another test's `send` takes
  the queued rows of a test and deletes them. Give the stores test its own database (a second
  `[db.*]` in `nvs.toml`) or its own `Stores` over a scratch connection, and a store folder of its
  own under `work/`.
- Stage 5 adds the verified-channel refusal to `Publish::record` and `App::addOwner`
  (`apps/registry/src/Publish.nvs`); there is no channel to verify before it.
- Stage 10's deploy guide names the `[capabilities.process] exec` grant of the `nvs` program, and
  an `fs.read` grant for each secret file outside the app folder; the committed `nvs.toml` has
  neither (standing decision *The build runs only where the operator grants it*).
- Changing a package's bound repository with a passkey is a route stage 4 or `registry-website` adds;
  `Publish` refuses the change with a 403 until then.
- Stage 4 writes the log entries for owner changes and for making and revoking a token; stage 5 notifies
  the owners and adds the verified-channel check to `addOwner` (`apps/registry/src/App.nvs`).
- Search, package pages, account pages and the policy page are goal `registry-website`'s (`docs/goals/registry-website.md`).
- The advisory feed and `nvs pkg audit` are goal `package-audit`'s (`docs/goals/package-audit.md`).
