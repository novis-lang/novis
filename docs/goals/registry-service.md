---
looper: 1
title: "the registry service does everything its rules describe, tested against a local registry"
summary: "every packaging rule's Not-on-disk part is built in apps/registry and nvs pkg, each is tested against a registry served locally, and no packaging rule has a Not-on-disk section left."
context: [docs/rules/packaging/the-registry-is-a-novis-program.md, apps/registry/README.md, crates/nvs-cli/src/package/mod.rs]
stages:
  - id: 1
    title: "nvs pkg fetch refuses an archive with an nvs.toml, and an extension package is packed and fetched as one"
    status: open
    context: [crates/nvs-cli/src/package/fetch.rs, crates/nvs-cli/src/package/pack.rs, crates/nvs-cli/src/ext.rs, docs/rules/packaging/a-package-is-its-digest.md, docs/rules/packaging/an-extension-package-carries-two-payloads.md]
  - id: 2
    title: "the registry logs an advisory against a range of versions, and a withhold purges the CDN"
    status: open
    context: [apps/registry/src/App.nvs, apps/registry/src/Releases.nvs, crates/nvs-cli/src/package/entry.rs, docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md, docs/rules/packaging/the-operator-only-withholds.md]
  - id: 3
    title: "nvs pkg advise sends an advisory, and nvs pkg audit reports it"
    status: open
    context: [crates/nvs-cli/src/package/audit.rs, crates/nvs-cli/src/package/retract.rs, crates/nvs-cli/tests/registry_build.rs, docs/reference/tools/35-packages.md]
  - id: 4
    title: "a person signs up, signs in and runs the account in a browser"
    status: open
    context: [apps/registry/src/AccountPage.nvs, apps/registry/src/BrowserTest.nvs, tools/nv/test/chromium.ts, docs/rules/packaging/a-registry-account-is-a-passkey.md, docs/rules/packaging/an-owner-is-notified-without-email.md]
  - id: 5
    title: "an owner confirms and changes a package in a browser, and no packaging rule has a Not-on-disk section"
    status: open
    context: [apps/registry/src/PackagePage.nvs, docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md, docs/rules/packaging/an-owner-may-unlist.md, docs/reference/tools/35-packages.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: fetch-and-extension-package-tests
    stage: 1
    name: fetch refuses an archive that carries an nvs.toml, and an extension package packs, fetches and runs as one
    run: [cargo, test, --test, package_fetch, --test, package_client]
    tests:
      - a_fetched_archive_that_carries_an_nvs_toml_is_refused_and_writes_nothing
      - an_nvs_toml_in_any_case_and_any_folder_is_refused_at_fetch
      - an_extension_package_packs_its_component_and_its_source
      - a_package_with_two_components_is_refused
      - a_component_whose_class_is_outside_the_package_namespace_is_refused
      - an_extension_package_is_fetched_and_its_extension_entry_is_printed
      - a_program_calls_an_extension_package_once_its_entry_is_written
  - id: advisory-entry-tests
    stage: 2
    name: an advisory entry carries a version range, the log refuses a malformed one, and the feed lists it
    run: [cargo, test, --test, registry_log, --test, package_audit]
    tests:
      - an_advisory_entry_with_a_closed_or_open_range_is_logged
      - an_advisory_entry_with_a_malformed_range_is_refused
      - the_feed_lists_an_advisory_entry_and_agrees_with_the_log
  - id: registry-app-tests
    stage: 2
    name: every test of apps/registry passes under nvs test, and the app serves
    run: [bun, nv, site, --check, apps]
    stdout_contains: ["site apps: ok"]
  - id: advise-and-audit-tests
    stage: 3
    name: nvs pkg advise waits seven days with a token and not with a passkey, and audit reports a locked version an advisory covers
    run: [cargo, test, --test, package_audit, --test, registry_build]
    tests:
      - audit_names_a_locked_version_an_advisory_covers_and_the_version_that_clears_it
      - audit_reports_an_advisory_with_no_fixed_version_and_no_fix
      - an_advisory_sent_with_a_token_is_pending_for_seven_days
      - an_advisory_confirmed_with_a_passkey_is_in_the_feed_at_once
  - id: advise-proofs
    stage: 3
    name: nvs pkg advise owes no feature proof
    run: [bun, nv, proofs, --id, "tools:packages/nvs-pkg-advise"]
    stdout_contains: ["complete."]
  - id: registry-browser-tests
    stage: 5
    name: a real Chromium with a virtual passkey signs up, runs the account and confirms a package on the registry's pages
    run: [bun, test, tools/nv/test/registry-browser.test.ts]
    env: { NV_BROWSER: "1" }
    tests:
      - a visitor signs up with a passkey and sees the account page
      - a person signs in with a passkey and with a recovery code
      - a signed-in person adds a second passkey
      - a person makes a token, sees it once and revokes it
      - a person adds a push device and verifies it from the test push
      - a person adds a webhook and verifies it with its code
      - an owner adds and removes a second owner
      - an owner releases a pending version now
      - an owner freezes the first stable API after reading it
      - an owner unlists a version and lists it again
      - an owner retracts a version and sends an advisory
      - an owner moves the package to another repository
  - id: no-not-on-disk
    stage: 5
    name: no packaging rule fragment has a Not-on-disk section
    run: [git, grep, -l, -F, "**Not on disk.**", --, docs/rules/packaging]
    exit: nonzero
    output_exact: ""
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, registry-service]
    reach: true
    floor: false
---

# the registry service does everything its rules describe, tested against a local registry

## Why

The first release ships with a package registry. Each packaging rule describes that registry, and
most of what the rules describe is built: `apps/registry/` serves search, the log, tokens, the
seven-day window, notices, feeds, retractions and withholds, and `nvs pkg` has every command the
rules name. What is left is a short list, and a release with any of it missing is a registry a
person cannot use:

- **Nobody can sign in from a browser.** The routes run every WebAuthn ceremony, and no page does.
  So no person can sign up, confirm a release, freeze an API, add a push device or answer a
  retraction notice on the website the rules send them to.
- **An advisory has no route.** The log's `advisory` entry exists, and no route writes one, so a
  publisher can only retract one version at a time.
- **Two client holes.** `nvs pkg fetch` unpacks an archive that carries an `nvs.toml`, which the
  registry never builds but a mirror can serve. An extension package is described and neither packed
  nor fetched as one.
- **A withheld archive stays on a CDN** until its cache lifetime ends.

Everything is built and tested against a registry served on this machine, with a key made on this
machine. Nothing is deployed.

## Standing decisions

**Scope.** This goal builds the parts of `docs/rules/packaging/` that are not built, as the stages
below list them. The audit behind the list (2026-10-30) found these sections stale, with their parts
already built: search, the log's second checkpoint copy, the token log entry and notice, the pending
feed and list, the repository route, the verified-channel refusal, the vendor case and `Novis\`
checks, the git-dependency refusal, `nvs pkg api`/`diff` and the minor-break refusal, the
subgraph report of `nvs pkg add`, the editor's `[grants]`, and `audit`/`publish`/`retract`/
`unlist`/`login`/`whoami`. `release-hygiene` rewrites those. This goal does not build them again.

**Out of scope: the production key.** `DEFAULT_KEY` (`crates/nvs-cli/src/package/manifest.rs:93`)
stays `None`. Writing it is the user's step at the release, in `release-run`. Every test uses a key
made locally with `nvs registry key`. It gets no gap record and no owner goal. Stage 5 only
rewrites the log rule's sentence about it as a present-tense fact: the default registry is read
once its verifier key is in the binary.

**The gate on `release-hygiene`.** Stage 1 starts once `bun nv owners --check` exits 0 and
`grep -rl registry-service data/gaps` lists the records `release-hygiene` wrote for this goal. Each
item below deletes its record as it lands. A record this goal has no item for gets an item in the
stage whose files it touches. An item with no record is built anyway: the rule is the spec.

**A notice goes through the channels the rules already chose.** `rule:packaging/an-owner-is-notified-without-email`
settles it: Web Push and webhooks, never email. Every new event in this goal writes its `notices`
row through `Events` in the change's transaction (`apps/registry/src/Events.nvs:20`). Tests reach a
webhook through `Test::answerHttp`, as `apps/registry/src/WebhooksTest.nvs:19` does. No mail
transport is added.

**An archive with an `nvs.toml` is refused at fetch.** `unpack_into`
(`crates/nvs-cli/src/package/fetch.rs:477`) refuses an entry whose last path part is `nvs.toml` in
any case, in any folder, with the same predicate `pack` uses (`crates/nvs-cli/src/package/pack.rs:421`),
moved into one `pub(crate)` function both call. The refusal names the file and writes nothing, and
the `.partial` folder is removed as on every other refusal. Home: `rule:packaging/a-package-cannot-reach-its-host`.

**An extension package.** Home: `rule:packaging/a-package-is-its-digest`, with a line in
`rule:packaging/an-extension-package-carries-two-payloads`.
- It is a package whose `files` list names exactly one `.nvsx`. `package.toml` gets no new key, and
  the archive, index and lock formats do not change. Two `.nvsx` files are refused.
- `nvs pkg pack` and `nvs registry build` load the component with `nvs_ext`'s loader
  (`crates/nvs-ext/src/load.rs:408`). They refuse a component that does not load, and a manifest
  class outside the package's namespace. The `Core\`/`Novis\` refusal is the loader's own.
- `nvs pkg fetch` lands it like any package. Then it prints, for each extension package that is new
  or has a new digest, the `[[extension]]` entry that loads it: the path under `vendor/` and the
  `sha256`, as `nvs ext pin` prints it (`crates/nvs-cli/src/ext.rs:339`). One function prints both.
- Fetch never writes `nvs.toml`. That file is the operator's
  (`rule:packaging/extension-loading-is-root-controlled`).
- Safe fallback: if the loader cannot run inside `pack`, the build alone checks the component and
  `pack` checks the count.

**An advisory.** Home: `rule:packaging/a-known-bad-version-is-retracted-not-deleted`.
- The entry is the existing `Entry::Advisory { id, package, versions, summary }`
  (`crates/nvs-cli/src/package/entry.rs:91`).
- `versions` is `A..B` (from `A`, up to `B` and not including it) or `A..` (from `A`, with no fixed
  version yet). `A` and `B` are full versions, and `A` is below `B`. One function parses it, and
  `Entry::parse`, `nvs registry log append`, audit and the registry's form all call it. There is no
  list and no other operator: a second range is a second advisory. A range system for dependencies
  stays declined (`rule:packaging/resolution-takes-the-highest-minimum`).
- `id` is `<vendor>/<name>-<n>`, where `n` counts that package's advisories from 1. The registry
  assigns it.
- The route is `POST /api/packages/{vendor}/{name}/advisory` with `{versions, summary}`. It has the
  access of the retract route (`apps/registry/src/App.nvs:826`): a token for the package waits seven
  days, and an owner's passkey from the last 15 minutes takes effect at once.
- The feed lists it: `is_advisory` (`crates/nvs-cli/src/package/advisory.rs:83`) takes
  `Entry::Advisory`. Audit reports each locked version inside the range, with the id, the summary
  and `B` as the version that clears it.
- Resolution does not change. An advisory is reported, and only a retraction or a withhold stops a
  version from being selected.
- The client is `nvs pkg advise <package> <versions> <summary>`, shaped like `nvs pkg retract`
  (`crates/nvs-cli/src/package/retract.rs:1`). It is a new feature, `tools:packages/nvs-pkg-advise`,
  with its own section in `docs/reference/tools/35-packages.md`.

**A withhold purges the CDN.** Home: `rule:packaging/the-operator-only-withholds`, and the settings
table in `apps/registry/README.md:16`.
- `hosting.json` gets an optional `purge`, in the shape of an `http` store:
  `{ "url": "...{url}...", "header": "...", "secret": "<file>" }`.
- After `Stores::remove` (`apps/registry/src/Stores.nvs:39`), the withhold job sends one `POST` per
  archive URL, under every entry of `hosts`, with `{url}` replaced. Bunny's purge API has this shape.
- Without `purge`, nothing is sent and the README says what a cache lifetime then means. A failed
  purge fails the job, and running it again sends it again, as `remove` already works.
- The purge goes through the outbound address table like a webhook.

**The registry's pages.** Home: `rule:packaging/the-registry-is-a-novis-program`, with a sentence in
each rule a page serves.
- The pages are server-rendered with `html` templates through `PackagePage::layout`
  (`apps/registry/src/PackagePage.nvs:503`). One same-origin script, `apps/registry/public/registry.js`,
  runs every WebAuthn ceremony and every `POST` and `DELETE` a page makes. A service worker,
  `apps/registry/public/push.js`, shows a push message and sends its code back. There is no
  framework, no build step and no third-party script.
- Each page sends `Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none'; frame-ancestors 'none'`
  from the one page helper (`apps/registry/src/App.nvs:1041`).
- The two scripts are files under `public/`, served by the static policy
  (`rule:http-server/static-serving-is-one-policy`). Safe fallback: a route answers each with
  `Core\Response::sendFile`.
- The pages call the routes that exist, with the session cookie and the bodies `BrowserTest` sends
  (`apps/registry/src/BrowserTest.nvs:13`). No route changes its access or its body for a page.
- An action that needs a passkey from the last 15 minutes first runs the sign-in ceremony when the
  route answers `403`, then sends the action again.
- New pages: `/signup` (the proof-of-work in the browser with `crypto.subtle`), and `/signin` with
  a passkey and a recovery-code form. The account page and the package and version pages gain
  forms and buttons.
- The page of `202` answers stays `/packages/<slug>`: the unlist and retract messages already name
  it.

**The browser tests.** `tools/nv/test/registry-browser.test.ts` runs Playwright's headless shell
through `tools/nv/test/chromium.ts:31`. It serves a copy of `apps/registry` with `nvs serve` on a
free port, as `crates/nvs-cli/tests/registry_build.rs:15` does, and its `hosting.json` sets
`signUpWork` to 4. A passkey is CDP's virtual authenticator (`WebAuthn.addVirtualAuthenticator`).
Headless Chromium has no push service, so `PushManager.prototype.subscribe` is replaced, through
`Page.addScriptToEvaluateOnNewDocument`, with a stub that returns a subscription whose keys the test
made. The test then reads the encrypted test push from the stub's endpoint, a `Test`-side receiver,
and hands the payload to the service worker. Each test waits on a page event or a response, never
on time. With no browser the suite skips, and the check sets `NV_BROWSER=1` so that a skip fails.

**Text an end user reads.** Every page, the two scripts' messages, every route message a page
shows, the help text and output of `nvs pkg advise`, the entry `nvs pkg fetch` prints, and each new
error follow AGENTS.md § *Text an end user reads*.

**Feature proofs** (`rule:testing/feature-proofs`). `tools:packages/nvs-pkg-advise` is new and owes
all of them. The fetch refusal, the extension package and the pages change features that are
complete today: `tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor`, `tools:packages/nvs-pkg-pack`,
`tools:packages/nvs-pkg-audit` and `tools:packages/the-registry-website`. Each gets the sentence in
its `about.md` and reference section, an example where the item says so, and
`bun nv proofs --id` still prints `complete.` for it. The withhold purge is a setting of the registry
app, not a feature, and owes no proof.

**Trade-offs.** Security: the fetch refusal and the extension check close two paths into a host,
and every page action keeps the passkey check its route already has. Latency: nothing on a
program's request path changes. A registry page is one query and one template, as now. Memory:
nothing per request beyond one page. An advisory is one row and one log record. Simplicity: one new
command, one new `hosting.json` key, two scripts, and no change to any file format.

**Lanes.** Stage 1 touches only `crates/nvs-cli/src/package/` and `crates/nvs-cli/src/ext.rs`, and
stage 2 touches `apps/registry/` and `entry.rs`/`advisory.rs`. They are disjoint and may run as
side goals. Stage 3 builds on stage 2's entry and route. Stages 4 and 5 share `App.nvs` with stage
2 and the page layout with each other, so they run after stage 2, in order.

## Stages

### 1. nvs pkg fetch refuses an archive with an nvs.toml, and an extension package is packed and fetched as one

**Does:** the client refuses a host's configuration file inside a package, and a package that
carries one `.nvsx` packs, fetches and loads as one extension package.

**Gate:** `bun nv owners --check` exits 0, and `grep -rl registry-service data/gaps` lists this
goal's records.

- [x] One `pub(crate)` predicate for "this path is a server configuration file", which `pack` and `unpack_into` both call. The refusal names the path and writes nothing. `rule:packaging/a-package-cannot-reach-its-host`. `crates/nvs-cli/src/package/pack.rs:421`, `crates/nvs-cli/src/package/fetch.rs:477`
- [x] The two `nvs.toml` tests the `fetch-and-extension-package-tests` check names, run once by name. They serve the archive from the fixture registry. `crates/nvs-cli/tests/package_fetch.rs:1`, `crates/nvs-cli/src/package/fixture.rs:1`
- [x] `docs/rules/packaging/a-package-cannot-reach-its-host.md:20` loses its Not-on-disk section, the body names the fetch check, and the feature's `about.md` says so. Delete the gap record. `bun nv proofs --id 'tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor'` prints `complete.`. `docs/examples/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/about.md:1`
- [ ] `pack` and `nvs registry build` find the one `.nvsx` in `files`, refuse a second, load it with the `nvs_ext` loader, and refuse a manifest class outside the package's namespace. `rule:packaging/a-package-is-its-digest`, `rule:packaging/an-extension-package-carries-two-payloads`. `crates/nvs-cli/src/package/pack.rs:193`, `crates/nvs-cli/src/package/build.rs:342`, `crates/nvs-ext/src/load.rs:408`
- [ ] `nvs pkg fetch` prints the `[[extension]]` entry of each new or changed extension package, through one function that `nvs ext pin` calls too. `rule:packaging/extension-loading-is-root-controlled`. `crates/nvs-cli/src/package/fetch.rs:231`, `crates/nvs-cli/src/ext.rs:339`
- [ ] The five extension-package tests the check names, on the fixture component `tests/conformance/ext/fixtures/ledger.nvsx`, run once by name. `crates/nvs-cli/tests/package_client.rs:181`, `crates/nvs-cli/tests/package_fetch.rs:292`
- [ ] A fourth fetch example: a vendored extension package and its `[[extension]]` entry in the example's `nvs.toml`. `bun nv proofs --comments` passes on it. The `pack` feature's `about.md` and reference section name the extension package. `docs/examples/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/04-a-package-with-a-component.nvs:1`, `docs/examples/tools/packages/nvs-pkg-pack/about.md:1`, `docs/reference/tools/35-packages.md:376`
- [ ] `docs/rules/packaging/a-package-is-its-digest.md:74` loses its Not-on-disk section, and the body states the extension package as the standing decision gives it. Delete the gap record. `bun nv proofs --id 'tools:packages/nvs-pkg-pack'` prints `complete.`. `docs/rules/packaging/an-extension-package-carries-two-payloads.md:13`

### 2. the registry logs an advisory against a range of versions, and a withhold purges the CDN

**Does:** an advisory is a log entry with a checked range, the registry has its route, and the
feed and the package page list it. A withhold also asks the CDN to drop the archive.

- [ ] The `versions` parser: `A..B` and `A..`, on the client's version type. `Entry::parse` refuses a malformed range. `rule:packaging/a-known-bad-version-is-retracted-not-deleted`. `crates/nvs-cli/src/package/entry.rs:91`, `crates/nvs-cli/src/package/manifest.rs:111`
- [ ] `is_advisory` takes `Entry::Advisory`, and `nvs registry log advisories` and `log verify` list it. `crates/nvs-cli/src/package/advisory.rs:83`, `crates/nvs-cli/src/registry.rs:169`, `crates/nvs-cli/src/registry.rs:394`
- [ ] The three tests the `advisory-entry-tests` check names, run once by name. `crates/nvs-cli/tests/registry_log.rs:1`, `crates/nvs-cli/tests/package_audit.rs:125`
- [ ] The route `POST /api/packages/{vendor}/{name}/advisory`, its `AdvisoryRequest`, `Releases::advise` with the seven days for a token, and `Events::advisory` with its log entry and notice in one transaction. `apps/registry/src/App.nvs:826`, `apps/registry/src/RetractRequest.nvs:1`, `apps/registry/src/Releases.nvs:177`, `apps/registry/src/Events.nvs:35`
- [ ] The package page and the version page show each advisory with its id, range and summary. `apps/registry/src/PackagePage.nvs:40`, `apps/registry/src/PackagePage.nvs:71`
- [ ] Registry tests: a token's advisory is pending, a passkey's is at once, a malformed range is `422`, and the owners get one notice. `apps/registry/src/AdvisoriesTest.nvs:1`
- [ ] `docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:45` and `docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md:46` state the advisory, its range and its route. The Not-on-disk sections' advisory parts go. Delete their gap records.
- [ ] `purge` in `Site`, and the withhold job's `POST` per archive URL after `Stores::remove`. `rule:packaging/the-operator-only-withholds`, `rule:packaging/registry-hosting-is-configuration`. `apps/registry/src/Site.nvs:64`, `apps/registry/jobs/withhold.nvs:1`, `apps/registry/src/Stores.nvs:39`
- [ ] Registry tests for the purge through `Test::answerHttp`: one `POST` per host, none without `purge`, and a failure fails the job. `apps/registry/src/StoresTest.nvs:1`
- [ ] The README's settings table names `purge`, and `docs/rules/packaging/the-operator-only-withholds.md:36` loses its Not-on-disk section. Delete its gap record. `apps/registry/README.md:16`
- [ ] The `registry-app-tests` check passes: `bun nv site --check apps`. `tools/nv/cmd/site.ts:1901`

### 3. nvs pkg advise sends an advisory, and nvs pkg audit reports it

**Does:** a publisher sends an advisory from the command line, and `nvs pkg audit` reports every
locked version it covers.

**Split:** the three examples and the attack of `tools:packages/nvs-pkg-advise` go to one writing
subagent once the command lands. It builds nothing.

- [ ] Audit reports each locked version inside an advisory's range, with its id, summary and fixed version, and exits non-zero. `rule:packaging/a-known-bad-version-is-retracted-not-deleted`. `crates/nvs-cli/src/package/audit.rs:194`, `crates/nvs-cli/src/package/audit.rs:218`
- [ ] The two audit tests the `advise-and-audit-tests` check names, run once by name. `crates/nvs-cli/tests/package_audit.rs:125`
- [ ] `nvs pkg advise <package> <versions> <summary>`: the command, its help text, and its token request, shaped like `retract`. `crates/nvs-cli/src/package/advise.rs:1`, `crates/nvs-cli/src/package/mod.rs:310`, `crates/nvs-cli/src/package/mod.rs:375`, `crates/nvs-cli/src/package/retract.rs:1`
- [ ] The two service tests the check names: a token's advisory against the served registry is pending, and a passkey's is in the feed at once. They run once by name. `crates/nvs-cli/tests/registry_build.rs:1418`
- [ ] The reference section `# nvs pkg advise`, after `nvs pkg retract and nvs pkg unlist`, and `nvs pkg audit`'s section names advisories. `docs/reference/tools/35-packages.md:665`, `docs/reference/tools/35-packages.md:707`
- [ ] The feature proofs of `tools:packages/nvs-pkg-advise`: `about.md`, three examples and one attack. `bun nv proofs --comments` passes on each, and the `advise-proofs` check prints `complete.`. `docs/examples/tools/packages/nvs-pkg-advise/about.md:1`, `tests/hostile/tools/packages/nvs-pkg-advise/01-a-stolen-token-sends-a-false-advisory.nvs:1`
- [ ] `bun nv proofs --id 'tools:packages/nvs-pkg-audit'` prints `complete.`, with its `about.md` naming advisories. `docs/examples/tools/packages/nvs-pkg-audit/about.md:1`

### 4. a person signs up, signs in and runs the account in a browser

**Does:** the registry has pages and one script for every passkey ceremony and every account
change, and a real browser test drives them.

**Split:** none. The pages share the layout, the script and the test file.

- [ ] The page helper sends the CSP, and the layout loads `registry.js`. `rule:packaging/the-registry-is-a-novis-program`. `apps/registry/src/App.nvs:1041`, `apps/registry/src/PackagePage.nvs:503`
- [ ] `/signup`: the proof-of-work with `crypto.subtle`, then the passkey registration. `/signin`: a passkey and a recovery-code form. `rule:packaging/a-registry-account-is-a-passkey`. `apps/registry/src/App.nvs:100`, `apps/registry/src/App.nvs:152`, `apps/registry/src/App.nvs:182`, `apps/registry/public/registry.js:1`
- [ ] The account page's forms: add a passkey, make and revoke a token (shown once), add and remove an owner, add and verify a webhook, sign out. Each runs the sign-in first when its route answers `403`. `apps/registry/src/AccountPage.nvs:30`, `apps/registry/src/App.nvs:267`, `apps/registry/src/App.nvs:307`, `apps/registry/src/App.nvs:370`, `apps/registry/src/App.nvs:475`
- [ ] Push: the account page subscribes with the key from `/api/push/key`, and `push.js` shows a message and sends its code back for a test push and for the "I am here" action. `rule:packaging/an-owner-is-notified-without-email`, `rule:packaging/an-account-stays-active-by-proof-not-by-token`. `apps/registry/src/App.nvs:427`, `apps/registry/src/App.nvs:519`, `apps/registry/src/App.nvs:239`, `apps/registry/public/push.js:1`
- [ ] Registry tests for what each page shows to a visitor, a recovery-code session and a passkey session. `apps/registry/src/AccountPageTest.nvs:1`
- [ ] The browser suite and its first seven tests from the `registry-browser-tests` check, run once by name with `NV_BROWSER=1`. `tools/nv/test/registry-browser.test.ts:1`, `tools/nv/test/chromium.ts:31`, `tools/nv/lib/shoot.ts:141`
- [ ] Every rule a page serves names it in one sentence. Delete the gap records of the sign-up, sign-in and push pages. `docs/rules/packaging/a-registry-account-is-a-passkey.md:11`, `docs/rules/packaging/an-owner-is-notified-without-email.md:1`

### 5. an owner confirms and changes a package in a browser, and no packaging rule has a Not-on-disk section

**Does:** each owner action on a package has its page or button, the rules describe only what is
built, and no gap names this goal.

- [ ] The version page: "Release now" for a pending version, and the stable-API page that shows the API from `.../api` before the freeze. `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`, `rule:packaging/only-a-major-may-break-the-api`. `apps/registry/src/PackagePage.nvs:71`, `apps/registry/src/App.nvs:722`, `apps/registry/src/App.nvs:732`, `apps/registry/src/App.nvs:741`
- [ ] The package page: unlist and relist, retract with a reason and a fixed-in version, the advisory form, and the repository move. `rule:packaging/an-owner-may-unlist`, `rule:packaging/the-registry-builds-the-archive-from-git`. `apps/registry/src/PackagePage.nvs:40`, `apps/registry/src/App.nvs:763`, `apps/registry/src/App.nvs:789`, `apps/registry/src/App.nvs:826`
- [ ] Registry tests for which buttons an owner, a non-owner and a visitor see. `apps/registry/src/PackagePageTest.nvs:1`
- [ ] The last five tests of the `registry-browser-tests` check, then the whole check once. `tools/nv/test/registry-browser.test.ts:1`
- [ ] `docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md:44` loses its Not-on-disk section. Delete its gap record.
- [ ] The website feature: its reference section and `about.md` describe sign-up, sign-in and the owner pages. `bun nv proofs --id 'tools:packages/the-registry-website'` prints `complete.`, and `bun nv site --stale` names no page this changed. `docs/reference/tools/35-packages.md:756`, `docs/examples/tools/packages/the-registry-website/about.md:1`
- [ ] `docs/rules/packaging/the-registry-keeps-an-append-only-log.md:96`: the `DEFAULT_KEY` sentence becomes a present-tense fact in the body: the default registry is read once its verifier key is in the binary. Nothing else changes for it. `rule:packaging/the-registry-keeps-an-append-only-log`. `crates/nvs-cli/src/package/manifest.rs:93`
- [ ] Every other packaging fragment `git grep -lF '**Not on disk.**' -- docs/rules/packaging` still lists is built or rewritten here, then `bun nv rules --render`. The `no-not-on-disk` check passes. `docs/rules/packaging.md:1`
- [ ] `bun nv owners --closes registry-service` passes. `tools/nv/cmd/owners.ts:1`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. The goal waits for `release-hygiene`, which turns each real Not-on-disk part
of `docs/rules/packaging/` into a gap record that names this goal, and rewrites the stale ones. The
audit behind this goal's list is in § *Standing decisions*, *Scope*.

`docs/rules/security/package-authority-is-granted-one-line-at-a-time.md:43` claims `nvs pkg add`
does not print the subgraph's requests. `crates/nvs-cli/src/package/report.rs:1` prints them, so
that section is stale and is `release-hygiene`'s.

### Next

Stage 1, files `crates/nvs-cli/src/package/{pack,fetch,build}.rs`, `crates/nvs-cli/src/ext.rs`,
`crates/nvs-cli/tests/package_{fetch,client}.rs`. Gate: `bun nv owners --check` exits 0, and
`grep -rl registry-service data/gaps` lists this goal's records.

- [ ] One server-configuration predicate for `pack` and `unpack_into`. `rule:packaging/a-package-cannot-reach-its-host`. `crates/nvs-cli/src/package/pack.rs:421`, `crates/nvs-cli/src/package/fetch.rs:477`
- [ ] The two `nvs.toml` fetch tests. `crates/nvs-cli/tests/package_fetch.rs:1`
- [ ] The rule's section, the feature's `about.md`, and the gap record. `docs/rules/packaging/a-package-cannot-reach-its-host.md:20`
- [ ] `pack` and `nvs registry build` check the one `.nvsx`. `rule:packaging/a-package-is-its-digest`. `crates/nvs-cli/src/package/pack.rs:193`, `crates/nvs-cli/src/package/build.rs:342`, `crates/nvs-ext/src/load.rs:408`
- [ ] Fetch prints the `[[extension]]` entry through `nvs ext pin`'s function. `rule:packaging/extension-loading-is-root-controlled`. `crates/nvs-cli/src/package/fetch.rs:231`, `crates/nvs-cli/src/ext.rs:339`
- [ ] The five extension-package tests. `crates/nvs-cli/tests/package_client.rs:181`, `crates/nvs-cli/tests/package_fetch.rs:292`
- [ ] The fourth fetch example and the `pack` feature's text. `docs/examples/tools/packages/nvs-pkg-fetch-and-nvs-pkg-vendor/04-a-package-with-a-component.nvs:1`, `docs/reference/tools/35-packages.md:376`
- [ ] The digest rule's section and its gap record. `docs/rules/packaging/a-package-is-its-digest.md:74`

No test is expected to go red.

### Backlog
