---
looper: 1
title: The registry website finds packages by what the registry witnessed
summary: "The registry's website searches by text and sorts by the reader's choice, with a default that mixes witnessed work and downloads by a formula published in the repository, every package page shows its versions and their states, requested capabilities and their changes, the API diff, the commit, the owners and their history, advisories, supersedes and every ranking fact, the account pages and the withhold policy page exist, the website has its feature proofs, rule packaging/search-sorts-by-the-readers-choice is shipped, and the checks below pass."
context: [apps/registry/README.md, apps/registry/src/App.nvs, docs/rules/packaging/search-sorts-by-the-readers-choice.md]
stages:
  - id: 1
    title: search matches text, and the reader chooses the sort
    status: open
    context: [apps/registry/src/Index.nvs, apps/registry/src/Downloads.nvs]
    extra:
      context:
        rules: [packaging/search-sorts-by-the-readers-choice]
  - id: 2
    title: a package page shows every fact the registry holds about it
    status: open
    context: [apps/registry/src/Releases.nvs, crates/nvs-cli/src/package/diff.rs, docs/rules/packaging/an-owner-may-unlist.md, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/search-sorts-by-the-readers-choice, packaging/only-a-major-may-break-the-api, packaging/an-owner-may-unlist, packaging/the-operator-only-withholds, packaging/a-release-waits-seven-days-unless-an-owner-confirms]
  - id: 3
    title: the account pages and the withhold policy page
    status: open
    context: [apps/registry/src/Accounts.nvs, apps/registry/src/Push.nvs, apps/registry/src/Tokens.nvs, docs/rules/packaging/the-operator-only-withholds.md]
    extra:
      context:
        rules: [packaging/a-registry-account-is-a-passkey, packaging/an-owner-is-notified-without-email, packaging/an-account-stays-active-by-proof-not-by-token, packaging/the-operator-only-withholds]
  - id: 4
    title: the website has its feature proofs, and its rule is shipped
    status: open
    context: [docs/reference/tools/35-packages.md]
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

**The formula is one file in the repository**, `apps/registry/src/Ranking.nvs`, and its doc comment
states it in words. Every package page shows every fact the formula reads, and links to that file.

**A package page shows** each version with its state (pending, unlisted, retracted or withheld), the
capabilities each version requests and what changed from the version before, the API diff per version,
the commit each version was built from, the owners and their history, advisories, `supersedes`, and the
facts behind ranking.

**The account pages** show the account's passkeys, its tokens with scope and expiry, its vendors and
co-owners, its push devices with the date each last answered, its webhooks, whether each channel is
verified and still counts, its inbox, and how many
days are left before it can no longer publish. The withhold policy page states, in a few short
paragraphs, that the operator only withholds, why, and that every withhold is a signed, logged
advisory.

**The pages are written for an end user** and follow AGENTS.md § *Text an end user reads*.

**Trade-offs.** No language feature changes. The pages are server-rendered by the registry's own
Novis program, so a page costs one request's memory. Search over a few thousand packages runs in SQLite
with its full-text index; a separate search service is not needed. Readers gain one sort they can
understand and change.

## Stages

### 1. search matches text, and the reader chooses the sort

**Does:** a query returns matching packages in the order the reader picked, and the default order
follows the published formula.

- [ ] Full-text search over names, summaries and doc comments. `apps/registry/src/Search.nvs:1`
- [ ] The ranking file and its facts, each computed from the registry's own records (`rule:packaging/search-sorts-by-the-readers-choice`). `apps/registry/src/Ranking.nvs:1`
- [ ] The five sorts, and a test that git history never changes an order. `apps/registry/src/SearchTest.nvs:1`

### 2. a package page shows every fact the registry holds about it

**Does:** one page per package, and one per version, show everything the standing decision lists.

- [ ] Versions and their states, the commit, and `supersedes`. `apps/registry/src/PackagePage.nvs:1`
- [ ] Requested capabilities and their change per version, and the API diff per version from `nvs pkg diff`. `apps/registry/src/PackagePage.nvs:1`, `crates/nvs-cli/src/package/diff.rs:1`
- [ ] Owners and their history, advisories, and the ranking facts. `apps/registry/src/PackagePage.nvs:1`

### 3. the account pages and the withhold policy page

**Does:** an owner manages everything about their account on its pages, and the policy page exists.

- [ ] Passkeys, tokens, vendors and co-owners, push devices and webhooks with their verification state, the inbox and the days left before publishing stops. `apps/registry/src/AccountPage.nvs:1`
- [ ] The withhold policy page (`rule:packaging/the-operator-only-withholds`). `apps/registry/src/PolicyPage.nvs:1`

### 4. the website has its feature proofs, and its rule is shipped

**Does:** the website is a feature with its proofs, and its rule is `shipped`.

- [ ] The heading `# The registry website` in `docs/reference/tools/35-packages.md`, owing nothing. `docs/reference/tools/35-packages.md:1`
- [ ] The attack: a package tries to rank higher with a backdated git history, and its order does not change. `tests/hostile/tools/packages/the-registry-website/01-a-backdated-history.nvs:1`
- [ ] The rule is `shipped`, and `bun nv owners --closes registry-website` passes. `data/rules/packaging/search-sorts-by-the-readers-choice.json:1`

## Handoff

### State

The goal is new. No stage is started. It needs goal `registry-service` reached.

### Next

Stage 1, search and ranking:
- full-text search (`rule:packaging/search-sorts-by-the-readers-choice`). `apps/registry/src/Search.nvs:1`
- the ranking file and its facts. `apps/registry/src/Ranking.nvs:1`

### Backlog

- Advisories on a package page come from the feed of goal `package-audit` (`docs/goals/package-audit.md`).
