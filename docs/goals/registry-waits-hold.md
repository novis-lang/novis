---
looper: 1
title: "a registry's waiting periods hold in the log, the feed and every command"
summary: "Reached when a retraction sent with a token enters the log and the advisory feed only after its seven days, an owner with a passkey cancels a waiting retraction or advisory, a lapsed account's token does nothing, and every command that is not a locked build refuses a signed file older than seven days."
context: [docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md]
stages:
  - id: 1
    title: "a retraction waits in the due column, and the log takes it from that time"
    status: done
    context: [apps/registry/src/Releases.nvs, apps/registry/src/Events.nvs, apps/registry/src/Index.nvs, apps/registry/src/ReleasesTest.nvs]
  - id: 2
    title: a token of an account that is no longer active does nothing
    status: done
    context: [apps/registry/src/Tokens.nvs, apps/registry/src/Publish.nvs, apps/registry/src/TokensTest.nvs, docs/rules/packaging/an-account-stays-active-by-proof-not-by-token.md]
  - id: 3
    title: an owner with a passkey cancels a waiting retraction or advisory
    status: done
    context: [apps/registry/src/App.nvs, apps/registry/src/Releases.nvs, apps/registry/src/Events.nvs, apps/registry/src/AdvisoriesTest.nvs]
  - id: 4
    title: "the package page, the attack and the pages a user reads show the wait and the cancel"
    status: done
    context: [apps/registry/src/PackagePage.nvs, apps/registry/public/registry.js, tests/hostile/README.md, docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md]
  - id: 5
    title: every command but a locked build refuses a signed file older than seven days
    status: open
    context: [crates/nvs-cli/src/package/index.rs, crates/nvs-cli/src/package/resolve.rs, crates/nvs-cli/src/package/proof.rs, docs/rules/packaging/the-registry-keeps-an-append-only-log.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: a-waiting-retraction-waits
    stage: 1
    name: "a retraction sent with a token is in neither the log nor the feed before its time, and a second one moves the waiting entry"
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry, --config, apps/registry/nvs.toml]
    timeout: 1800
    tests:
      - aRetractionByTokenShowsSevenDaysLaterAndByPasskeyAtOnce
      - aRetractionByTokenIsInNeitherTheLogNorTheFeedBeforeItsTime
      - aWaitingRetractionCountsInNoRankingFact
  - id: a-waiting-retraction-waits-in-the-store
    stage: 1
    name: "the served registry's feed does not list a retraction that nvs pkg retract sent with a token"
    run: [cargo, test, --test, registry_build, --, a_retraction_sent_with_a_token_is_pending_for_seven_days]
    timeout: 1800
    tests:
      - a_retraction_sent_with_a_token_is_pending_for_seven_days
  - id: transactions-write-first
    stage: 1
    name: "every transaction of the registry application starts with a statement that writes"
    run: [cargo, test, --test, registry_sources]
    timeout: 1800
    tests:
      - every_registry_transaction_starts_with_a_write
  - id: a-lapsed-token-does-nothing
    stage: 2
    name: "a token of a lapsed account, or of one with no verified channel, retracts, advises and publishes nothing"
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry, --config, apps/registry/nvs.toml]
    timeout: 1800
    tests:
      - aTokenOfALapsedAccountDoesNothing
      - aTokenOfAnAccountWithNoVerifiedChannelDoesNothing
      - anInactiveAccountIsRefusedUntilAPasskeySignIn
  - id: an-owner-cancels
    stage: 3
    name: "an owner with a recent passkey cancels a waiting retraction or advisory, a token cancels nothing, and an entry in the log stays"
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry, --config, apps/registry/nvs.toml]
    timeout: 1800
    tests:
      - anOwnerCancelsAWaitingRetractionAndTheLogNeverHasIt
      - anOwnerCancelsAWaitingAdvisoryAndItsIdIsNotUsedAgain
      - aTokenCancelsNothing
      - anEntryInTheLogIsNotCancelled
  - id: the-page-shows-the-wait-to-owners
    stage: 4
    name: "the package page shows a waiting retraction or advisory to an owner with its cancel form, and to nobody else"
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry, --config, apps/registry/nvs.toml]
    timeout: 1800
    tests:
      - onlyAnOwnerSeesAWaitingRetractionOrAdvisory
  - id: old-signed-files-are-refused
    stage: 5
    name: "an index file signed more than seven days ago is refused unless the build is locked"
    run: [cargo, test, --bin, nvs, --, a_signed_file_older_than_seven_days_is_refused_unless_the_build_is_locked]
    timeout: 1800
    tests:
      - a_signed_file_older_than_seven_days_is_refused_unless_the_build_is_locked
  - id: old-files-through-the-binary
    stage: 5
    name: "add and a fetch that is not locked refuse an old index file, and a locked fetch reads it"
    run: [cargo, test, --test, package_client, --, an_old_index_file_is_refused_for_every_command_but_a_locked_fetch]
    timeout: 1800
    tests:
      - an_old_index_file_is_refused_for_every_command_but_a_locked_fetch
  - id: audit-refuses-an-old-feed
    stage: 5
    name: "nvs pkg audit fails on an advisory feed signed more than seven days ago and names the registry and the time"
    run: [cargo, test, --test, package_audit, --, audit_refuses_an_advisory_feed_signed_more_than_seven_days_ago]
    timeout: 1800
    tests:
      - audit_refuses_an_advisory_feed_signed_more_than_seven_days_ago
---

# a registry's waiting periods hold in the log, the feed and every command

## Why

The registry service is a Novis application under `apps/registry/`. A publish token sits in CI and is
the credential most likely to leak, so everything a token sends waits seven days, and every owner gets
a message. `rule:packaging/a-known-bad-version-is-retracted-not-deleted` promises that a stolen token
cannot retract a good release "before every owner has been notified and has had time to answer".
`rule:packaging/the-registry-keeps-an-append-only-log` promises that a file host can serve an old copy
of the signed files for seven days at most. Five places do not keep these promises.

- **A token's retraction is public and permanent in the same request.** `App::retract` gives a token
  `$from = now + 7 days` (`apps/registry/src/App.nvs:906`). `Releases::retract` stores it in
  `retracted_at` and logs through `Events::reasoned` (`apps/registry/src/Releases.nvs:189`), whose
  entry has no `due` time (`apps/registry/src/Events.nvs:35`, `apps/registry/src/Events.nvs:96`).
  `Index::append` takes every entry whose `due` is null (`apps/registry/src/Index.nvs:104`), and the
  route flushes at once (`apps/registry/src/App.nvs:912`). The log and the advisory feed list the
  retraction before the request ends. Only the index writer reads `retracted_at`. `nvs pkg audit` then
  fails on every lock that has the version and prints the sender's reason. A record in the log cannot
  be removed. `Releases::retract` called with a time seven days ahead, on a database in memory with
  this tree's release build, leaves an `events` row whose `due` is null. An advisory already has the
  right shape: `Events::advisory` inserts its entry with `due` (`apps/registry/src/Events.nvs:52`).
- **An owner has seven days and nothing to do in them.** No handler sets `retracted_at` back to null
  or deletes a waiting `events` or `advisories` row. The only writes are
  `apps/registry/src/Releases.nvs:182`, `:209` and `:219`.
- **A lapsed account's token still retracts and advises.** `Publish::publish` checks
  `Activity::isActive` and `Channels::hasVerified` by hand (`apps/registry/src/Publish.nvs:44`).
  `App::retract`, `App::advise` and the unlist route reach a token only through `Tokens::allows`
  (`apps/registry/src/App.nvs:902`, `:956`, `:1038`), which checks neither
  (`apps/registry/src/Tokens.nvs:101`). `rule:packaging/an-account-stays-active-by-proof-not-by-token`
  calls the 60 days "a bound on how long a leaked token stays useful".
- **Two commands accept a signed file of any age.** The seven-day bound is `Registries::selecting`
  (`crates/nvs-cli/src/package/resolve.rs:166`), and only `add`, `update` and `outdated` call it
  (`crates/nvs-cli/src/package/minimum.rs:87`). `nvs pkg audit` and an `nvs pkg fetch` without
  `--locked` do not (`crates/nvs-cli/src/package/audit.rs:139`,
  `crates/nvs-cli/src/package/fetch.rs:221`). A checkpoint carries no time, and `Stamp::read` refuses
  a time ahead and never a time behind (`crates/nvs-cli/src/package/index.rs:262`). A file host that
  serves a checkpoint, an index and a feed from before a retraction gets a clean audit on a computer
  with no kept checkpoint, such as a CI runner. A fetch that writes a new lock entry takes a retracted
  version from the old index with no message.
- **`Releases::advise` starts its transaction with a read** (`apps/registry/src/Releases.nvs:203`).
  SQLite fails the write that follows at once while another request holds the write lock, and the
  owner gets a `500`. Commit `2b4d679d16` fixed this shape in `Vendors::removeOwner`,
  `Releases::rebind` and `Activity::answer`, and `advise` was written later.

The existing tests pin the defects. `ReleasesTest::aRetractionByTokenShowsSevenDaysLaterAndByPasskeyAtOnce`
counts three `retract` records in the log (`apps/registry/src/ReleasesTest.nvs:153`), and one of the
three was sent with the later time. `an_old_index_file_is_refused_only_for_selecting`
(`crates/nvs-cli/src/package/index.rs:982`) pins the split between commands. The attack
`tests/hostile/tools/packages/nvs-pkg-retract-and-nvs-pkg-unlist/01-a-retraction-by-token-alone.nvs`
models the handler with a class of its own, so it never sees the log.

## Standing decisions

**A waiting event waits in one place, the `due` column.** `Events::reasoned` takes the `due` time
`Events::advisory` already takes, and returns the id of the waiting entry as `advisory` does.
`Releases::retract` passes its `$from`, and `Releases::withhold` passes now. `Index::append` is not
changed: it already takes an entry only from its `due` time, and the daily run adds an entry whose time
has come (`apps/registry/src/Index.nvs:184`). **What it spends:** nothing per request. Declined: a
second filter in `Index::append` that reads `retracted_at`. Two columns would then say when one event
starts.

**A second retraction moves the waiting entry and adds none.** `release_states` gains the column
`retract_event`, the id of the waiting row in `events`, as `advisories.event` is for an advisory.
`Schema` adds it to an existing database the way it adds `events.due`
(`apps/registry/src/Schema.nvs:175`). When a retraction moves the time forward, `Releases::retract`
rewrites that entry's text and `due`, because the second sender may give another reason. It writes a
notice, so every owner sees that the version is retracted from the new time. **What it spends:** one
integer per retracted version. Safe default: when the update of the waiting entry changes no row, the
retraction adds a new entry with its `due`. Declined: finding the waiting entry by a search in its
JSON text.

**The wait holds in every reader of `retracted_at`.** The readers are the index writer
(`apps/registry/src/Index.nvs:264`), the page's state (`apps/registry/src/PackagePage.nvs:192`), the
page's *Advisories* list (`apps/registry/src/PackagePage.nvs:402`) and a ranking fact
(`apps/registry/src/Ranking.nvs:62`). The first two compare with now. The other two do not: the list
is the page decision below, and the ranking fact counts a retraction from its time. `Index::run` and
`Index::daily` read the clock once and pass it to `append` and `write`, so an index file never shows a
retraction whose entry the same run did not add (`apps/registry/src/Index.nvs:103`,
`apps/registry/src/Index.nvs:252`).

**An owner cancels a waiting retraction or advisory on the website, with a recent passkey.** One
route: `POST /api/packages/<vendor>/<name>/cancel`. Its body is `{"version": "1.4.0"}` for a
retraction or `{"advisory": "acme/http-1"}` for an advisory, and exactly one of the two is set. The
caller must be an owner of the vendor who signed in with a passkey in the last 15 minutes
(`App::ownsWithPasskey`, `apps/registry/src/App.nvs:1065`). The transaction starts with its write: it
deletes the waiting row of `events`. One deleted row means the entry was not in the log yet. It then
clears `retracted_at`, `retract_reason`, `fixed_in` and `retract_event`, or marks the advisory row
cancelled, and writes a notice to the owners. No deleted row means the log has the entry, and the
route answers `409`. The answers, in the words a user reads:

| Case | Status | `message` or `error` |
|---|---|---|
| a retraction cancelled | 200 | `The retraction of version 1.4.0 is cancelled.` |
| an advisory cancelled | 200 | `The advisory acme/http-1 is cancelled.` |
| the entry is in the log | 409 | `This retraction is in the public log already. It cannot be cancelled.` |
| nothing waits | 404 | `No retraction of this version is waiting.` |
| no recent passkey, or not an owner | 403 | `Sign in with a passkey as an owner of the package first.` |
| both fields or neither | 422 | `Send a version or the id of an advisory, and not both.` |

The advisory rows use "advisory" and its id in the same sentences. **What it spends:** nothing per
request on any other route. Declined: a cancel through `nvs pkg` with a token. A token is what the wait
protects against, so a token must not end the wait in either direction. Declined: cancelling an entry
that is in the log. The log is append-only, and a retraction that is wrong is answered by a new
version.

**A cancel is not a log entry.** The log's list of entry kinds is closed, and a cancelled entry never
entered the log, so the log has nothing to correct. The notice is the record: `notices` rows are never
deleted, and the vendor feed and the inbox read them. Its kind is `cancel`. Its text is ``An owner
cancelled the retraction of `Acme\Http` 1.4.0. It did not start.`` or `An owner cancelled the advisory
acme/http-1. It was not listed.` Declined: a `cancel` entry kind. It would make a stolen token's false
retraction a permanent public record, which is what the wait prevents.

**A cancelled advisory keeps its row, so its id is never given to another advisory.**
`Releases::advise` numbers an advisory by the count of the package's rows
(`apps/registry/src/Releases.nvs:214`). Deleting row 1 of 2 would give the next advisory the id of
row 2. So `advisories` gains `cancelled_at`, the cancel sets it, the count still includes the row, and
every reader skips it: the same-advisory lookup in `advise` and the page's list. The same advisory sent
again after a cancel is a new advisory with a new id. **What it spends:** one short row per cancelled
advisory, kept. Declined: numbering by the highest id. It reuses the id of a cancelled last advisory,
which the owners' notices already named.

**The package page shows a waiting retraction or advisory to an owner alone.** Today
`PackagePage::advisories` lists a retraction and an advisory with their text from the moment they are
sent, to every visitor (`apps/registry/src/PackagePage.nvs:402`, `apps/registry/src/PackagePage.nvs:412`).
That publishes a stolen token's text on the package's own page for the seven days. A visitor sees each
from its time. An owner sees a part *Waiting changes* with each waiting one, its time and its cancel
form. **What it spends:** nothing. Declined: showing a waiting one to everybody as "waiting". Unlike a
pending release, nobody needs to test it before it starts. The vendor feed keeps the notice, because
the notice is how the owners learn of it.

**A token of an account that is no longer active does nothing.** One function,
`Tokens::standing(Token $token): ?string`, returns the sentence that says why the token's account may
not use a token now, or `null`. It holds the two checks `Publish::publish` has today: the account
showed activity in the last 60 days, and it has a verified channel. `Tokens::allows` calls it, and
`allows` is the one function every token use but `whoami` passes. `Publish::publish` loses its own two
checks and asks `standing` after `find`, so it still answers `403` with the sentence before the build
starts. `PublishTest::anInactiveAccountIsRefusedUntilAPasskeySignIn` holds that the build is never
called. `App::retract` and `App::advise` keep their one `401` sentence for every token `allows`
returns `null` for. **What it spends:** two indexed reads per token request, one row of `accounts` by
its key and one row of `channels` by `channels_of_account`. Declined: a check in each route. The next
route with a token would forget it, as two did. Declined: moving the checks into `Tokens::find`.
`whoami` must still answer for a lapsed account, so `nvs pkg whoami` can say whose token it is.

**Age is judged where a signed file is opened, not per command.** One function on `Stamp` judges the
age and writes the message. `Registry::checked` asks it where it compares today
(`crates/nvs-cli/src/package/index.rs:645`), and `Seen::advisories` asks it after `covers`
(`crates/nvs-cli/src/package/proof.rs:226`). `Registries` holds the bound from `new`, and one builder,
`from_the_lock`, clears it. `fetch` and `vendor` call it under `--locked` and nothing else does.
`Registries::selecting` and its four calls go. There is no new setting and no new limit: the bound is
the existing `index::MAX_AGE`. The root file is still never refused for its age, so the kept copy finds
the file hosts through an outage. `nvs registry log verify` passes no bound, because an operator checks
a log of any age. **What it spends:** nothing in time or memory. It costs usability in two cases while
a registry signs nothing for more than seven days. `nvs pkg audit` fails with a message that says the
registry's files are old, where today it reports nothing found. `nvs pkg fetch` without `--locked`
fails with the same message, and the message names `--locked`, which still builds from the lock.
Declined: a bound only on a fetch that writes a new lock entry. The age would then be judged inside
resolution, per version, and the next command would be forgotten as `audit` was. Declined: a time in
the checkpoint. The rule declines it, and the feed and each index file already carry one.

**A refusal for age is loud and says what to do.** It is an error that ends the command, and its first
lines are these. Each host's own failure follows on its own line, as now.

```text
The index of `Acme\Http` from the registry `https://registry.example.com` is too old to use.
It was signed on 2026-10-28 14:05 UTC, and `nvs pkg` needs a file signed in the last seven days.
Try again later. `nvs pkg fetch --locked` still downloads the versions in `package.lock`.
```

```text
The advisory feed of the registry `https://registry.example.com` is too old to use.
It was signed on 2026-10-28 14:05 UTC, and `nvs pkg audit` needs a feed signed in the last seven days.
The command cannot tell you that no advisory applies. Try again later.
```

A session may change a word to fit `AGENTS.md` § *Text an end user reads*. The message always names the
registry's URL, the signing time in UTC, the seven days and the next step. A file host with a fresh
file is still tried after one with an old file.

**`nvs registry log advisories` takes `--at <SECONDS>`,** the option `nvs registry index sign` has
(`crates/nvs-cli/src/registry.rs:124`), with the same help text. A test then writes an old feed with
the binary. Declined: signing the old feed inside the test. The test would hold a second copy of the
feed's format.

**`Releases::advise` starts its transaction with its write,** as the methods `2b4d679d16` fixed do
(`apps/registry/src/Releases.nvs:163`). The fix left a sibling because nothing reads the next
transaction somebody writes. So one test reads every `transaction(` in `apps/registry/src/` outside the
test classes, and fails naming a method whose first statement on `$tx` is a `query`. Each method it
names is fixed the same way. **What it spends:** test time only. Safe default: where a transaction only
reads, it stops being a transaction, or it gets the same first statement.

**The proof is a test in `apps/registry`, and a second attack runs the real method.** An attack under
`tests/hostile/` has no expected output and passes whatever it prints (`tests/hostile/README.md`
§ *The contract*). So no attack can hold that the log stays empty. The assertions are `#[Test]` methods
of the application, which the checks run with `nvs test apps/registry --config apps/registry/nvs.toml`,
the command the goal that built the service used. An attack can still load the application: a program
at the repository root that writes `autoload 'Registry' from '<path>/apps/registry/src'` opens a
database in memory, calls `Registry\Schema::setUp` and `Registry\Releases::retract`, and reads the
`events` table. This ran on this tree's release build. So:

- `01-a-retraction-by-token-alone.nvs` stays a model. It is about the route's checks, and a route
  needs a request, a session and the application's own database. Its text follows the new answers.
- `02-a-retraction-that-enters-the-log-early.nvs` is new and loads the application. It calls
  `Releases::retract` with the time a token gets and prints what the log may add now, what it may add
  seven days later, and what is left after `Releases::cancelRetraction`. It reads the waiting entries
  through `Events::ready`, the query `Index::append` takes its entries from, moved into `Events` so
  both call one function.

Safe default: if the proofs gate or the website render refuses an attack that loads files outside its
own folder, file `02` models the `events` table in a class of its own, and its top comment says that
`apps/registry/src/ReleasesTest.nvs` holds the real check. The attack on `nvs pkg audit` is a model in
both cases, because the client is Rust and a `.nvs` file cannot call it. Every comment in these files
follows `AGENTS.md` § *Text an end user reads*.

**No feature id is new.** `bun nv proofs --id` reports `tools:packages/the-registry-website`,
`tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist` and `tools:packages/nvs-pkg-audit` complete, and
the cancel route belongs to the first two. So the proofs gate owes nothing new. The goal still adds two
attacks and the sentences in each `about.md` and in the reference page, because each of them describes
the wait.

**Lanes:** two lanes that may run side by side. The service lane is stages 1 to 4, in order: stages
1, 3 and 4 share `apps/registry/src/Releases.nvs`, `Events.nvs` and `PackagePage.nvs`, and stages 2
and 3 share `App.nvs`. The client lane is stage 5. It touches `crates/nvs-cli/src/package/`,
`crates/nvs-cli/src/registry.rs`, `crates/nvs-cli/tests/package_client.rs` and `package_audit.rs`, and
`docs/rules/packaging/the-registry-keeps-an-append-only-log.md`, and the service lane touches none of
them. The lanes share one file, `docs/reference/tools/35-packages.md`: stage 4 edits its `nvs pkg
retract` and `nvs pkg advise` parts and stage 5 its signed-files and `nvs pkg audit` parts, so the lane
that lands second edits it after the first one's commit. This goal follows `package-authority-closed`
on the chain and needs none of its work.

## Stages

### 1. a retraction waits in the due column, and the log takes it from that time

**Does:** give a retraction's log entry its `due` time, keep one waiting entry per retraction, hold the
wait in the ranking fact, and start every transaction with a write.

- [x] `Events::reasoned` takes `int $due` and returns the id of the entry, and `Events::add` inserts
  `due` and returns the id (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`,
  `apps/registry/src/Events.nvs:35`, `apps/registry/src/Events.nvs:96`). The class comment is
  rewritten as a whole.
- [x] `Events::ready(Connection|Transaction $db, int $now)` holds the query for the entries the log
  may add at `$now`, and `Index::append` calls it (`apps/registry/src/Index.nvs:104`).
- [x] `release_states` gains `retract_event`, added to an existing database as `events.due` is
  (`apps/registry/src/Schema.nvs:135`, `apps/registry/src/Schema.nvs:175`).
- [x] `Releases::retract` passes `$from` as `due` and stores the entry's id. A retraction that moves
  the time forward rewrites the waiting entry's text and `due`, adds no entry, and writes a notice
  (`apps/registry/src/Releases.nvs:177`, `apps/registry/src/Releases.nvs:189`). Its doc comment says
  when the log takes the entry.
- [x] `Releases::withhold` passes now (`rule:packaging/the-operator-only-withholds`,
  `apps/registry/src/Releases.nvs:241`).
- [x] `Index::run` and `Index::daily` read the clock once and pass it to `append` and `write`
  (`apps/registry/src/Index.nvs:103`, `apps/registry/src/Index.nvs:252`).
- [x] The ranking fact counts a retraction from its time
  (`rule:packaging/search-sorts-by-the-readers-choice`, `apps/registry/src/Ranking.nvs:62`). Test `aWaitingRetractionCountsInNoRankingFact` sits beside
  `everyFactOfTheScoreIsOnThePage` (`apps/registry/src/PackagePageTest.nvs:83`).
- [x] `ReleasesTest::aRetractionByTokenShowsSevenDaysLaterAndByPasskeyAtOnce` changes its expectation
  (`apps/registry/src/ReleasesTest.nvs:132`). Before the week the log double has no line. After it the
  log has one `retract`. The token retraction of `0.0.10` and the passkey one that moves it leave one
  row in `events`, and the last line expects `['retract', 'retract']`
  (`apps/registry/src/ReleasesTest.nvs:153`).
- [x] Test `aRetractionByTokenIsInNeitherTheLogNorTheFeedBeforeItsTime` in
  `apps/registry/src/AdvisoriesTest.nvs:129`: after a flush `log.txt` does not exist and
  `Log::advisories` was never called, and after seven days the log has the entry and the feed is
  written once.
- [x] Test `a_retraction_sent_with_a_token_is_pending_for_seven_days`, beside the advisory's
  (`crates/nvs-cli/tests/registry_build.rs:1584`): `nvs pkg retract` with a token against the served
  registry, and the feed in its store does not name the version.
- [x] `Releases::advise` starts with a statement that writes, with the comment `rebind` has
  (`apps/registry/src/Releases.nvs:203`, `apps/registry/src/Releases.nvs:163`).
- [x] Test `every_registry_transaction_starts_with_a_write` in
  `crates/nvs-cli/tests/registry_sources.rs:1`. It reads every `.nvs` file under `apps/registry/src/`
  and `apps/registry/jobs/` that is not a test class, through `nvs_repo::path`. Each method it names
  is fixed in this stage.
- [x] Rule fragment: a retraction's entry has a `due` time as an advisory's has, and neither the log
  nor the feed has it before then
  (`docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:55`).
  `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms` says the same in its last
  paragraph (`docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md:40`). Then
  `bun nv rules --render`.

### 2. a token of an account that is no longer active does nothing

**Does:** move the activity check and the verified-channel check into `Tokens`, where every token use
but `whoami` passes them.

- [x] `Tokens::standing(Token $token): ?string` returns the `403` sentence of each failed check, in
  the words `Publish::publish` uses today, or `null`
  (`rule:packaging/an-account-stays-active-by-proof-not-by-token`, `apps/registry/src/Tokens.nvs:101`,
  `apps/registry/src/Activity.nvs:37`, `apps/registry/src/Channels.nvs:126`).
- [x] `Tokens::allows` returns `null` when `standing` returns a sentence, before it saves the time of
  use. Its doc comment lists the four things a token needs (`apps/registry/src/Tokens.nvs:97`).
- [x] `Publish::publish` drops its two checks and answers `403` with `standing`'s sentence right after
  `find`, before the build (`apps/registry/src/Publish.nvs:44`). `Publish::record` keeps its `allows`
  call (`apps/registry/src/Publish.nvs:124`).
- [x] Test `aTokenOfALapsedAccountDoesNothing` in `apps/registry/src/TokensTest.nvs:107`, under a
  fixed clock: after 61 days `allows` returns `null`, and the retract, advisory and unlist routes each
  answer as they do for a token that is not valid. A passkey sign-in makes the token work again.
- [x] Test `aTokenOfAnAccountWithNoVerifiedChannelDoesNothing` beside it: the account's channel is
  deleted, and `allows` returns `null`.
- [x] The tests that call `allows` today are in `TokensTest` and at
  `apps/registry/src/ActivityTest.nvs:48`. Each uses an account that is active and has a verified
  channel, so one follows only where its clock is past 60 days at the call. A test that sends a token
  for an account inserted at `active_at` 0 gets an active account from `BrowserTest::newAccount`
  (`apps/registry/src/BrowserTest.nvs:75`).
- [x] Rule fragments: an account that lapsed can use no token for anything, and `Tokens::allows` is
  where that is checked (`docs/rules/packaging/an-account-stays-active-by-proof-not-by-token.md:1`,
  `docs/rules/packaging/an-account-stays-active-by-proof-not-by-token.md:31`).
  `rule:packaging/a-publish-token-is-scoped-and-expires` lists what a token may send, and says a token
  stops when its account lapses (`docs/rules/packaging/a-publish-token-is-scoped-and-expires.md:13`,
  `docs/rules/packaging/a-publish-token-is-scoped-and-expires.md:27`). Then `bun nv rules --render`.

### 3. an owner with a passkey cancels a waiting retraction or advisory

**Does:** add the cancel to `Releases` and its one route, with the answers the standing decision
lists. Gate: stage 1 landed, so `release_states.retract_event` exists.

- [x] `advisories` gains `cancelled_at`, added to an existing database as `events.due` is
  (`apps/registry/src/Schema.nvs:181`).
- [x] `Releases::cancelRetraction(string $key, string $version): bool` and
  `Releases::cancelAdvisory(string $key, string $id): bool`. Each starts its transaction with the
  delete of the waiting `events` row, returns `false` when no row was deleted, clears or marks its
  row, and writes the `cancel` notice (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`,
  `apps/registry/src/Releases.nvs:192`, `apps/registry/src/Releases.nvs:225`).
- [x] `Events::cancelled` writes the notice, with the two texts of the standing decision
  (`rule:packaging/an-owner-is-notified-without-email`, `apps/registry/src/Events.nvs:47`).
- [x] `Releases::advise` skips a cancelled row in its same-advisory lookup and still counts it for the
  next id (`apps/registry/src/Releases.nvs:203`, `apps/registry/src/Releases.nvs:214`).
- [x] `CancelRequest`, a `#[Core\Json\Derive]` class with `version` and `advisory`, shaped as
  `RetractRequest` is (`apps/registry/src/CancelRequest.nvs:1`).
- [x] `App::cancel`, the route `POST /api/packages/{vendor}/{name}/cancel`, after `App::advise`
  (`apps/registry/src/App.nvs:975`). It answers `403` unless `App::ownsWithPasskey` holds and never
  reads a token. It tells an entry in the log from nothing waiting by `retracted_at`, sends the
  waiting notices, and answers with the table's sentences.
- [x] The token answers of `App::retract` and `App::advise` name the cancel: `An owner can retract it
  now or cancel it with a passkey on the website.` and the same for an advisory
  (`apps/registry/src/App.nvs:915`, `apps/registry/src/App.nvs:970`). Their doc comments follow.
- [x] Test `anOwnerCancelsAWaitingRetractionAndTheLogNeverHasIt` in
  `apps/registry/src/ReleasesTest.nvs:157`: a token retraction, the cancel, seven days, a flush. The
  log double has no line, the index shows `"released"`, and the owners have one `cancel` notice.
- [x] Test `anOwnerCancelsAWaitingAdvisoryAndItsIdIsNotUsedAgain` in
  `apps/registry/src/AdvisoriesTest.nvs:92`: two advisories, the first cancelled, and the next one
  sent gets the id `acme/http-3`. The same advisory sent again is a new one.
- [x] Tests `aTokenCancelsNothing` and `anEntryInTheLogIsNotCancelled` through the route, beside
  `aTokenCannotUnlistAndGetsThePageOfThePasskeyStep` (`apps/registry/src/ReleasesTest.nvs:294`). The
  first sends the cancel with a token and no session, then with a session that has no recent passkey,
  and gets `403` both times. The second cancels a passkey retraction and gets `409`.

### 4. the package page, the attack and the pages a user reads show the wait and the cancel

**Does:** show an owner what waits with its cancel form, hide it from everybody else until its time,
and bring the attack, the descriptions and the rules to the new behaviour. Gate: stage 3 landed, so
`App::cancel` answers.

**Split:** one writing subagent writes the two attack files, and one the three `about.md` files with
the reference page. Their files are disjoint from the session's, which writes the page, the script and
the tests. Neither subagent builds.

- [x] `PackagePage::advisories` lists a retraction from its time and an advisory from its time, and
  never a cancelled advisory (`rule:packaging/search-sorts-by-the-readers-choice`,
  `apps/registry/src/PackagePage.nvs:402`, `apps/registry/src/PackagePage.nvs:412`).
- [x] `PackagePage::waiting`, shown to an owner above the forms of `manage`
  (`apps/registry/src/PackagePage.nvs:133`). Its text:
  `<h2>Waiting changes</h2>` and `A retraction or an advisory that was sent with a token waits seven
  days. You can cancel it before it starts. After that, it is in the public log and cannot be
  cancelled.` Each row is `The retraction of 1.4.0 starts on 2026-06-22. The reason is: <reason>` or
  `The advisory <id> about the versions <range> starts on 2026-06-22. <summary>`, with a form
  `data-action="cancel"` whose button says `Cancel the retraction` or `Cancel the advisory`. The part
  is left out when nothing waits. The class comment names the part.
- [x] The step `cancel` in `registry.js` sends the route through `withPasskey`
  (`apps/registry/public/registry.js:221`).
- [x] Test `onlyAnOwnerSeesAWaitingRetractionOrAdvisory` in `apps/registry/src/PackagePageTest.nvs:187`:
  an owner's page has both rows and both forms, a visitor's page has neither text, and after seven
  days the visitor's page lists both.
- [x] A case `an owner cancels a waiting retraction` in the browser suite, after the retract case
  (`tools/nv/test/registry-browser.test.ts:605`). CI's browser job and `bun nv deep` run that suite,
  and this stage does not.
- [x] Attack `tests/hostile/tools/packages/nvs-pkg-retract-and-nvs-pkg-unlist/02-a-retraction-that-enters-the-log-early.nvs:1`,
  as the standing decision describes, with one line above each numbered step.
- [x] Attack `tests/hostile/tools/packages/nvs-pkg-retract-and-nvs-pkg-unlist/01-a-retraction-by-token-alone.nvs:103`
  gains two steps in its model: a token cancels a waiting retraction, and the token of an account with
  no activity for 61 days retracts. Both are stopped.
- [x] `docs/examples/tools/packages/nvs-pkg-retract-and-nvs-pkg-unlist/about.md:12` and
  `docs/examples/tools/packages/nvs-pkg-advise/about.md:9` say that an owner can cancel it on the
  website before the seven days end. `docs/examples/tools/packages/the-registry-website/about.md:11`
  names the waiting changes among what an owner sees.
- [x] `docs/reference/tools/35-packages.md:713` and `docs/reference/tools/35-packages.md:758` say the
  same for `nvs pkg retract` and `nvs pkg advise`, and the website part says what an owner sees
  (`docs/reference/tools/35-packages.md:826`). Then `bun nv render --website`.
- [x] Rule fragments: `rule:packaging/a-known-bad-version-is-retracted-not-deleted` states the cancel
  route, who may call it, that a token cannot, that a cancel is a notice and no log entry, and that a
  cancelled advisory keeps its id. Its Declined list gains the cancel by token and the `cancel` entry
  kind (`docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:5`,
  `docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:72`).
  `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms` names the *Waiting changes*
  part beside the "Release now" button
  (`docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md:50`). Then
  `bun nv rules --render`.
- [x] `bun nv proofs --comments` over the two attack files and the three `about.md` files, and every
  line it names is written again from what the code does.

### 5. every command but a locked build refuses a signed file older than seven days

**Does:** judge the age of an index file and of the advisory feed where each is opened, with one
message, and leave only a locked fetch outside the bound.

- [ ] One function on `Stamp` returns the age refusal for a file signed before a given time, with the
  message of the standing decision (`rule:packaging/the-registry-keeps-an-append-only-log`,
  `crates/nvs-cli/src/package/index.rs:262`). `Registry::checked` calls it
  (`crates/nvs-cli/src/package/index.rs:645`). The module doc and `MAX_AGE`'s doc say which callers
  pass no bound (`crates/nvs-cli/src/package/index.rs:31`, `crates/nvs-cli/src/package/index.rs:77`).
- [ ] `Registries::new` sets `oldest` to now less `MAX_AGE`, `from_the_lock` clears it, and
  `selecting` goes with its four calls (`crates/nvs-cli/src/package/resolve.rs:154`,
  `crates/nvs-cli/src/package/resolve.rs:166`, `crates/nvs-cli/src/package/minimum.rs:87`,
  `crates/nvs-cli/src/package/minimum.rs:153`, `crates/nvs-cli/src/package/minimum.rs:183`,
  `crates/nvs-cli/src/package/minimum.rs:243`).
- [ ] `fetch` calls `from_the_lock` when `locked` is set, for `nvs pkg fetch` and `nvs pkg vendor`
  (`crates/nvs-cli/src/package/fetch.rs:221`).
- [ ] `Seen::advisories` refuses a feed older than the bound, after `covers`
  (`crates/nvs-cli/src/package/proof.rs:225`). `nvs pkg audit` then ends with the feed's message, and
  its index read is bounded by `Registries` (`crates/nvs-cli/src/package/audit.rs:154`,
  `crates/nvs-cli/src/package/audit.rs:166`).
- [ ] `nvs registry log advisories` takes `--at <SECONDS>` (`crates/nvs-cli/src/registry.rs:176`), and
  the help of `nvs registry index sign` says every command but a locked fetch needs a file from the
  last seven days (`crates/nvs-cli/src/registry.rs:110`).
- [ ] The help of `nvs pkg audit` says it needs a feed signed in the last seven days, and the help of
  `--locked` says it works while the registry's files are old
  (`rule:packaging/help-text-speaks-to-its-reader`, `crates/nvs-cli/src/package/mod.rs:154`,
  `crates/nvs-cli/src/package/mod.rs:191`, `crates/nvs-cli/src/package/mod.rs:246`).
- [ ] Test `a_signed_file_older_than_seven_days_is_refused_unless_the_build_is_locked` replaces
  `an_old_index_file_is_refused_only_for_selecting` (`crates/nvs-cli/src/package/index.rs:982`). It
  also holds that a file signed one second inside the bound is read.
- [ ] Test `an_old_index_file_is_refused_for_every_command_but_a_locked_fetch` replaces
  `an_old_index_file_is_refused_for_add_and_read_for_a_locked_fetch`
  (`crates/nvs-cli/tests/package_client.rs:732`). `add` and a `fetch` without `--locked` fail, the
  message names the registry's URL, ` UTC`, `seven days` and `--locked`, no lock entry is written, and
  `fetch --locked` succeeds.
- [ ] Test `audit_refuses_an_advisory_feed_signed_more_than_seven_days_ago`
  (`crates/nvs-cli/tests/package_audit.rs:467`). The feed is written with `--at` eight days back
  (`crates/nvs-cli/tests/package_audit.rs:91`), the command fails, and the message names the registry
  and the time. A feed signed now over the same log passes.
- [ ] Attack `tests/hostile/tools/packages/nvs-pkg-audit/02-an-old-advisory-feed.nvs:1`, a model as
  `01-a-hidden-advisory.nvs` is. A file host serves a feed and a log from before a retraction, then
  the same feed with its time changed to now, then a feed signed far in the future. Each is stopped,
  and a feed signed today passes.
- [ ] `docs/reference/tools/35-packages.md:522` says which commands need a file from the last seven
  days and that `--locked` does not, and `docs/reference/tools/35-packages.md:792` says what `nvs pkg
  audit` prints for an old feed. `docs/examples/tools/packages/nvs-pkg-audit/about.md:1` and
  `docs/examples/tools/packages/the-transparency-log/about.md:14` say it in one sentence each. Then
  `bun nv render --website`.
- [ ] Rule fragment: every command that is not a locked build refuses an index file or a feed signed
  more than seven days ago, and the Declined entry on refusing every command names the locked fetch as
  what still works (`docs/rules/packaging/the-registry-keeps-an-append-only-log.md:33`,
  `docs/rules/packaging/the-registry-keeps-an-append-only-log.md:78`). Then `bun nv rules --render`.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done, each stage's checks passing, run once by name. The service lane is finished.

Stage 4: `PackagePage::waiting` (`apps/registry/src/PackagePage.nvs:137`) shows an owner each
retraction with `retracted_at` later than now and each advisory with `shows_at` later than now and
`cancelled_at` null, each with a `data-action="cancel"` form. `advisories` lists both only from
their time and never a cancelled advisory. The step `cancel` in `apps/registry/public/registry.js`
sends `{"version"}` or `{"advisory"}` by `data-advisory`. The browser case `an owner cancels a
waiting retraction` uses a new job `retract.nvs` in the suite's job map; it was not run (CI's browser
job and `bun nv deep` run it). Attack `02-a-retraction-that-enters-the-log-early.nvs` loads the real
application with `autoload 'Registry' from '../../../../../apps/registry/src'`, the form the
`tests/hostile/package/` attacks use, and ran clean on the release binary. Attack `01` gained the
61-day token as step 1 and the token cancel as step 10.

The sweep's reds are stale, as the last handoff said: `env-secret-proofs` has no check now, and
nightly 43001191130 is the run already traced to a test `305c04f660` rewrote.

### Next

Stage 5, the client lane, over `crates/nvs-cli/src/package/`, `crates/nvs-cli/src/registry.rs`,
`crates/nvs-cli/tests/package_client.rs`, `package_audit.rs` and
`docs/rules/packaging/the-registry-keeps-an-append-only-log.md`. Its items in the stage list carry the
anchors. Stage 4 already edited `docs/reference/tools/35-packages.md` (the retract, advise and website
parts), so stage 5 edits its signed-files part (`:522`) and audit part (about `:795` now).

- [ ] The `Stamp` age function and `Registry::checked` (`crates/nvs-cli/src/package/index.rs:262`, `:645`).
- [ ] `Registries` holds the bound, `from_the_lock` clears it, `selecting` goes (`crates/nvs-cli/src/package/resolve.rs:154`).
- [ ] `Seen::advisories` (`crates/nvs-cli/src/package/proof.rs:225`) and `--at` on `nvs registry log advisories`.
- [ ] The three tests the stage names, each run once by name (`crates/nvs-cli/src/package/index.rs:982`,
  `crates/nvs-cli/tests/package_client.rs:732`, `crates/nvs-cli/tests/package_audit.rs:467`), then the attack and the words.

Expected red until stage 5 lands: none. Stage 5 replaces `an_old_index_file_is_refused_only_for_selecting`
and `an_old_index_file_is_refused_for_add_and_read_for_a_locked_fetch` with its own tests.

### Backlog

- `Publish::publish` answers a retry before it checks the token's scope
  (`apps/registry/src/Publish.nvs:74`). Owner: `rule:packaging/publishing-never-overwrites`.
- Adding a passkey from a recovery-code session writes no notice (`apps/registry/src/App.nvs:319`).
  Owner: `rule:packaging/an-owner-is-notified-without-email`.

The open items `package-authority-closed` left when it was retired:

- Grant enforcement through a callable, a `mixed` receiver, inheritance and namespace identity is not
  reviewed. Owner: `rule:security/authority-is-the-enclosing-namespace`.
- `require` of an application file from a package namespace was not traced.
- `Core\Ldap\Connection::authenticate` dials again with no `require`: for a `connect`-made connection
  it resolves the host anew through `walk`, with no address policy
  (`crates/nvs-stdlib/src/ldap/mod.rs:495`, `:598`).
- `Core\Cli`'s prompt opens the terminal device by name with `std::fs` and no door
  (`crates/nvs-runtime/src/terminal.rs:632`).
- `Core\Session`'s file store opens SQLite in the data folder with no door
  (`crates/nvs-stdlib/src/session/sqlite.rs:111`).
- `Core\Crypto` signs with `ring`'s `SystemRandom`, which a test seed does not reach
  (`crates/nvs-stdlib/src/crypto.rs:2794`, `:3027`).
- The scan of `no_class_with_a_reason_for_none_calls_a_door` sees a door called in a class's module.
  A `NEEDS_NONE` class that reaches a door through a helper in another module, with a forwarded
  label, is not seen (`crates/nvs-stdlib/tests/capability.rs`).
- A package's `Core\Config::get` of a key that is not a secret directive needs `env` only because a
  grant line is asked per member. Asking it only for a literal secret key would let a package read
  `limits.*` with no line (`crates/nvs-types/src/capability.rs:224`).
- An array literal of the program's own ints, such as `[1, 2]`, passed to a method of a `mixed`
  receiver is refused as `E0895`, which calls it `mixed` text. The refusal is in the safe direction,
  but `rule:security/taint-propagation` says such a literal carries no outside text
  (`report_args_with_no_parameter_list`, `crates/nvs-types/src/expr/calls.rs`, with
  `check_array_literal`).
- A property-path or string-with-holes failure still names the bad segment's text, such as
  ``has no public property `$nmae` `` (`nvs_property_path_mismatch`, `nvs_string_holes_mismatch`,
  `crates/nvs-runtime/src/helpers.rs`). Name the segment by position instead.
- `$fn->call($obj, $a)` on a typed callable checks only `$a`'s qualifier against the written
  parameter, not its type (`infer_callable_rebind`, `crates/nvs-types/src/expr/calls.rs`).
- A `.=` compound write through an erased property is not asked the untyped-write question, only the
  plain `=` (`crates/nvs-types/src/expr/assign.rs:1083`).
- A third-party extension method that takes a resource may return text built from an earlier
  `tainted` argument as plain text (`crates/nvs-types/src/ext_lib.rs:290`). Not traced to a run.
- `slot_check` and `test_shape` are two maps from a type to a run-time test. An intersection type
  and `iterable` stay `SlotCheck::Any` in the first, and a single-value `string` keeps its tag alone
  (`crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- A callable's `invoke` row carries no parameter type spellings, so a bare `callable`'s refusal
  names no type (`crates/nvs-ir/src/lower/anon_fn.rs:374`, `crates/nvs-runtime/src/dispatch.rs:286`).
- `check_param_class` runs on every call of an anonymous function, a proven one too
  (`crates/nvs-ir/src/lower/anon_fn.rs:454`).
- `Core\Str::format('%1$s', $o)` over a `Stringable` object throws "it does not implement
  `Stringable`": `stringify` finds no compiled `toString` (`crates/nvs-runtime/src/helpers.rs:2049`).
  Once it works, `crates/nvs-codegen/tests/borrowed_reads.rs` gains a row for it.
