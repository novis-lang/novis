---
looper: 1
title: The registry website finds packages by what the registry witnessed
summary: "The registry's website searches by text and sorts by the reader's choice, with a default that mixes witnessed work and downloads by a formula published in the repository, every package page shows its versions and their states, requested capabilities and their changes, the API diff, the commit, the owners and their history, advisories, supersedes and every ranking fact, the account pages and the withhold policy page exist, the website has its feature proofs, rules packaging/search-sorts-by-the-readers-choice and packaging/the-registry-is-a-novis-program are shipped, and the checks below pass."
context: [apps/registry/README.md, apps/registry/src/App.nvs, docs/rules/packaging/search-sorts-by-the-readers-choice.md]
stages:
  - id: 1
    title: search matches text, and the reader chooses the sort
    status: done
    context: [apps/registry/src/Index.nvs, apps/registry/src/Releases.nvs, crates/nvs-cli/src/package/build.rs]
    extra:
      context:
        rules: [packaging/search-sorts-by-the-readers-choice]
  - id: 2
    title: a package page shows every fact the registry holds about it
    status: open
    context: [apps/registry/src/Releases.nvs, apps/registry/src/Ranking.nvs, crates/nvs-cli/src/package/diff.rs, docs/rules/packaging/an-owner-may-unlist.md, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/search-sorts-by-the-readers-choice, packaging/only-a-major-may-break-the-api, packaging/an-owner-may-unlist, packaging/the-operator-only-withholds, packaging/a-release-waits-seven-days-unless-an-owner-confirms]
  - id: 3
    title: the account pages and the withhold policy page
    status: open
    context: [apps/registry/src/Accounts.nvs, apps/registry/src/Push.nvs, apps/registry/src/Tokens.nvs, apps/registry/src/Channels.nvs, apps/registry/src/Activity.nvs, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/a-registry-account-is-a-passkey, packaging/an-owner-is-notified-without-email, packaging/an-account-stays-active-by-proof-not-by-token, packaging/the-operator-only-withholds]
  - id: 4
    title: the website has its feature proofs, and its rule is shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
  - id: 5
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: registry-tests
    stage: 4
    name: the registry application's own tests pass
    run: [cargo, run, -q, --bin, nvs, --, test, apps/registry]
  - id: website-proofs
    stage: 4
    name: the registry website owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "tools:packages/the-registry-website"]
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, registry-website]
    reach: true
    floor: false
---

# The registry website finds packages by what the registry witnessed

## Why

The service from goal `registry-service` publishes, confirms and records, and its pages are still
bare. This goal builds what a reader sees: search, the package page and the account pages. Ranking is
the part that shapes behaviour. A registry that ranks by stars or commit counts rewards gaming and
one-function helpers. This one ranks by facts it saw itself, and shows each one, so a reader can judge
a package on evidence.

## Standing decisions

**Search matches text, then sorts by the reader's choice.** The sorts are the default, by date, by
activity, by downloads and by name. The default mixes witnessed work and downloads.

**Witnessed work is only what the registry saw with its own clock or measured in the code it built:**
age on the registry, months with at least one release, time since the last release, public API size
(from `nvs pkg api`, goal `api-compat`), doc-comment coverage, and retractions that name a fixed-in
version. Git history (first commit date, commit count) is shown as "from the repository, not checked"
and never ranks.

**Doc-comment coverage is produced by `nvs registry build`.** Its JSON gains `doc_coverage`: the
public declarations that carry a doc comment, over all public declarations, counted in the same API
extraction the build already runs for the diff. The service stores it on the release row, and
`Ranking.nvs` reads it from there. This goal adds the field (stage 1). Why: the registry measures it
in code it built itself, and one extraction serves the diff and the count.

**The formula is one file in the repository**, `apps/registry/src/Ranking.nvs`, and its doc comment
states it in words. Every package page shows every fact the formula reads, and links to that file.

**A package page shows** each version with its state (pending, unlisted, retracted or withheld), the
capabilities each version requests and what changed from the version before, the API diff per version,
the commit each version was built from, the owners and their history, advisories, `supersedes`, the
facts behind ranking, the readme from the archive, every file in the archive with its size, and every
external download with its host, size, SHA-256 and license
(`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`).

**The account pages** show the account's passkeys, its tokens with scope and expiry, its vendors and
co-owners, its push devices with the date each last answered, its webhooks, whether each channel is
verified and still counts, its inbox, and how many
days are left before it can no longer publish. The withhold policy page states, in a few short
paragraphs, that the operator only withholds, why, and that every withhold is a signed, logged
advisory.

**The pages are written for an end user** and follow AGENTS.md § *Text an end user reads*.

**Lanes.** Stage 1 (`Search.nvs`, `Ranking.nvs`, `build.rs`), stage 2 (`PackagePage.nvs`) and stage 3
(`AccountPage.nvs`, `PolicyPage.nvs`) create disjoint files and share only the route wiring in
`apps/registry/src/App.nvs`, so they can run in any order and merge serially on that file. Stage 2's
ranking facts read `Ranking.nvs`, so that one item waits for stage 1. Goal `package-audit`'s
`Advisories.nvs` also wires a route in `App.nvs`, so its merge is serial with this goal's. Stage 4
writes `docs/reference/tools/35-packages.md`, where every package goal adds a heading.

**Gates.** Every stage needs goal `registry-service` reached: `docs/goals/registry-service.md` is gone.
Stage 2's advisories need `apps/registry/src/Advisories.nvs` (goal `package-audit`); until it exists,
the page shows withholds and retractions from `Releases.nvs` and the advisory item stays open. A
session whose gate does not hold ends `BLOCKED` naming the missing file.

**Trade-offs.** No language feature changes. The pages are server-rendered by the registry's own
Novis program, so a page costs one request's memory. Search over a few thousand packages runs in SQLite
with its full-text index; a separate search service is not needed. Readers gain one sort they can
understand and change. The `doc_coverage` count costs the build one pass over declarations it already
holds.

**A package's summary is the first paragraph of its readme**, which `nvs registry build` prints as
`summary` (at most 280 bytes), beside `docs`, the text of the public doc comments (at most 32 KiB,
`api::MAX_DOCS_TEXT`). Why: `package.toml` has no summary key, and adding one grows the manifest
for a fact the archive already carries. The build's JSON is `schema` 2 with both keys and
`doc_coverage` as `{documented, public}`. The full-text index `search` holds the highest version's
text, one row per package.

**The release row keeps `declarations` and `documented`.** `declarations` is also the public API
size the ranking reads, so one count serves both facts. `Schema::setUp` adds the two columns to a
database made before them; a release restored from the log has both null, read as 0.

**Search is `GET /api/search?q=&sort=`**, which returns each package with every fact `Ranking.nvs`
reads. A query keeps its first eight runs of letters and digits, each a prefix match, so no FTS
operator reaches the index. The facts are computed per request in one query, which is
O(packages matched); a cached score table waits until a few thousand packages make it measurable.

## Stages

### 1. search matches text, and the reader chooses the sort

**Does:** a query returns matching packages in the order the reader picked, and the default order
follows the published formula.

**Split:** an `Explore` subagent lists the release and download columns the service's schema holds,
one line each with its anchor. After the session lands `Ranking.nvs`, a writing subagent writes
`SearchTest.nvs`.

- [x] `nvs registry build` reports `doc_coverage`, and the service stores it on the release row. `crates/nvs-cli/src/package/build.rs:1`, `apps/registry/src/Publish.nvs:1`
  - [x] `api::extract_counted` counts the documented public declarations in the API walk.
  - [x] The build prints `doc_coverage`, and `registry_build.rs` checks it.
  - [x] `Publish` stores it, and `Schema::setUp` adds the columns to an older database.
- [x] Full-text search over names, summaries and doc comments. `apps/registry/src/Search.nvs:1`
  - [x] The build prints `summary` and `docs`.
  - [x] The FTS5 table `search`, written by a publish of the highest version.
  - [x] `Search::matchOf` and the `/api/search` route.
- [x] The ranking file and its facts, each computed from the registry's own records (`rule:packaging/search-sorts-by-the-readers-choice`). `apps/registry/src/Ranking.nvs:1`
  - [x] `Ranking::facts`, one query over the records.
  - [x] `Ranking::score`, with the formula in its doc comment.
- [x] The five sorts, and a test that git history never changes an order. `apps/registry/src/SearchTest.nvs:1`
  - [x] Each sort's order, operator characters, matched fields, and withheld and pending releases.
  - [x] `gitHistoryNeverChangesTheOrderOrAFact`.

### 2. a package page shows every fact the registry holds about it

**Does:** one page per package, and one per version, show everything the standing decision lists.

**Split:** after the session lands the page and its route, one writing subagent writes the
capability and API diff sections, then a second writes the test.

- [ ] Versions and their states, the commit, and `supersedes`. `apps/registry/src/PackagePage.nvs:1`
- [ ] Requested capabilities and their change per version, and the API diff per version from `nvs pkg diff`. `apps/registry/src/PackagePage.nvs:1`, `crates/nvs-cli/src/package/diff.rs:1`
- [ ] Owners and their history, advisories, and the ranking facts with the link to `Ranking.nvs`. `apps/registry/src/PackagePage.nvs:1`
- [ ] The readme, rendered as untrusted markdown, the archive's file list with sizes, and the external downloads per version. `apps/registry/src/PackagePage.nvs:1`
- [ ] A test: a package with a pending, an unlisted, a retracted and a withheld version shows each state, and every fact `Ranking.nvs` reads is on the page. `apps/registry/src/PackagePageTest.nvs:1`

### 3. the account pages and the withhold policy page

**Does:** an owner manages everything about their account on its pages, and the policy page exists.

**Split:** after the session lands the route wiring, one writing subagent writes `AccountPage.nvs`
and its test, then a second writes `PolicyPage.nvs`.

- [ ] Passkeys, tokens, vendors and co-owners, push devices and webhooks with their verification state, the inbox and the days left before publishing stops. `apps/registry/src/AccountPage.nvs:1`
- [ ] A test: the page needs a passkey sign-in, and shows an unverified channel and the days left under a fixed clock. `apps/registry/src/AccountPageTest.nvs:1`
- [ ] The withhold policy page (`rule:packaging/the-operator-only-withholds`). `apps/registry/src/PolicyPage.nvs:1`

### 4. the website has its feature proofs, and its rule is shipped

**Does:** the website is a feature with its proofs, and its rule is `shipped`.

**Split:** after the session writes the heading, one writing subagent writes `about.md` and the
examples `bun nv proofs --id 'tools:packages/the-registry-website'` lists, then a second writes the
attack.

- [ ] The heading `# The registry website` in `docs/reference/tools/35-packages.md`. `docs/reference/tools/35-packages.md:1`
- [ ] `about.md` and the examples the proofs command lists. `docs/examples/tools/packages/the-registry-website/about.md:1`
- [ ] The attack: a package tries to rank higher with a backdated git history, and its order does not change. `tests/hostile/tools/packages/the-registry-website/01-a-backdated-history.nvs:1`
- [ ] The rule is `shipped` with `SearchTest.nvs` in its `guardedBy`, and `bun nv owners --closes registry-website` passes. `data/rules/packaging/search-sorts-by-the-readers-choice.json:1`
- [ ] `rule:packaging/the-registry-is-a-novis-program` is `shipped` with `apps/registry/src/App.nvs` in its `guardedBy`: goal `registry-service` built the service, and its title also says the website is part of `apps/registry/`, which this goal makes true. `data/rules/packaging/the-registry-is-a-novis-program.json:1`

### 5. the goal is verified once, and every failure is fixed

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

Stage 1 is done. `nvs registry build` prints `doc_coverage`, `docs` and `summary` (schema 2), and
the release row keeps `declarations` and `documented`. A publish of a package's highest version
writes the FTS5 table `search`. `GET /api/search?q=&sort=` returns the facts `Ranking.nvs` reads,
in the order the reader picked. `SearchTest.nvs` has five passing tests. The CI failure in
`ext_world.rs` is fixed: the two `Core\WebPush` classes are in `REFUSED`.

Tests that may go red in stage 5: none expected. `registry_build.rs` and `PublishTest.nvs` were
updated for schema 2, and only their changed tests ran.

### Next

Stage 2, the package page, on `apps/registry/src/PackagePage.nvs` and a route in `App.nvs`.
- [ ] Versions and their states, the commit, and `supersedes`. `apps/registry/src/Releases.nvs:1`
- [ ] Capabilities per version and their change, and the API diff from the stored API files. `apps/registry/src/Publish.nvs:106`, `crates/nvs-cli/src/package/diff.rs:1`
- [ ] Owners and their history (`notices`), and the ranking facts from `Ranking::facts`. `apps/registry/src/Ranking.nvs:1`
- [ ] The readme, file list and external downloads. The build prints `files` but the row does not keep them yet. `crates/nvs-cli/src/package/build.rs:216`
- [ ] The page's test. `apps/registry/src/PackagePageTest.nvs:1`

### Backlog

- Advisories on a package page come from the feed of goal `package-audit` (`docs/goals/package-audit.md`).
