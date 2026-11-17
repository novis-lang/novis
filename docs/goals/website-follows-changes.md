---
looper: 1
title: "every change to Novis reaches the website: stale on a behaviour change, listed on What changed, and read by a newcomer"
summary: "Reached when a feature's tests are part of the hash that makes its about.md, help and covering pages stale, bun nv site --check changes fails on an added, removed or changed feature with no What changed entry, bun nv site --check review fails on a changed user text with no review stamp, the rule and the Reviewed proof are written, and the tree is green under all three."
context: [tools/nv/cmd/site.ts, docs/rules/testing/website-pages-are-stamped.md, docs/rules/testing/feature-proofs.md]
stages:
  - id: 1
    title: "a change to a feature's tests makes its pages, its about.md and its help stale"
    status: open
    context: [tools/nv/proofs/markers.ts, tools/nv/test/site.test.ts, data/rules/testing/website-pages-are-stamped.json, data/rules/testing.json]
  - id: 2
    title: "an added, removed or changed feature needs an entry in changes.json, and a release moves the entries onto What changed"
    status: open
    context: [website/src/content/docs/in-depth/what-changed.mdx, website/src/lib/feature.ts, website/src/components/MethodChangelog.astro, tools/nv/schema/proofs.ts, .github/workflows/release.yml, tools/nv/cmd/ci.ts]
  - id: 3
    title: every feature changed since the last release has its entry
    status: open
    context: [website/src/content/docs/in-depth/what-changed.mdx, docs/agent/prose.md]
  - id: 4
    title: "a changed user text needs a review stamp, and Reviewed is a feature proof"
    status: open
    context: [tools/nv/proofs/collect.ts, tools/nv/schema/proofs.ts, data/proofs/policy.json, tools/nv/cmd/proofs.ts, docs/agent/prose.md, .claude/agents/reader.md]
  - id: 5
    title: every goal names what it owes the website, and its gate stage runs the review
    status: open
    context: [AGENTS.md, docs/agent/conventions.md, docs/agent/loop-authoring.md, docs/agent/prose.md, docs/agent/commands.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
    context: [docs/agent/prose.md]
checks:
  - id: feature-hash
    stage: 1
    name: "a feature's covering tests are in its page hash, its about.md and help card are stamped texts, the baseline runs once, and the search questions keep their hash"
    run: [bun, test, tools/nv/test/site.test.ts]
    timeout: 900
    tests:
      - a changed test that covers a feature makes its pages, its about.md and its help stale
      - a Rust test's text runs from its covers marker to its closing brace
      - a test that only calls a member is not in its hash
      - an about.md is stamped against everything but itself, and a help card against everything but the card
      - the baseline stamps every about.md and help card once, restamps a page whose stamp was current, and refuses a second run
      - a page's search questions keep the hash without tests
  - id: stale-green
    stage: 1
    name: "no page, about.md or help card is stale or broken on the tree"
    run: [bun, nv, site, --check, stale]
    timeout: 900
  - id: changes-tool
    stage: 2
    name: "an added, removed or changed feature with no entry is named, an entry is held to its shape, and the release moves the entries onto What changed, the feature changelog and a new snapshot"
    run: [bun, test, tools/nv/test/site-changes.test.ts]
    timeout: 900
    tests:
      - an added, a removed and a changed feature with no entry are each named
      - an entry naming the feature clears it
      - an unseen record clears a feature only at the hash it names
      - a removed entry and a breaking entry with no instead text are named
      - an entry over three sentences or outside the prose bounds is named
      - an entry naming a feature neither the roster nor the snapshot has is named
      - a highlight entry may name a feature whose hash did not move
      - the release moves each entry under its version heading on What changed and empties the file
      - the release adds each entry to the changelog of every feature it names
      - the release writes a new snapshot named after its version
  - id: changes-green
    stage: 3
    name: "every feature added, removed or changed since the snapshot is named by an entry"
    run: [bun, nv, site, --check, changes]
    timeout: 900
  - id: review-tool
    stage: 4
    name: "every user text is listed with its key and hash, a changed one with no stamp is named, the baseline excuses only an unchanged text, and Reviewed is owed by a feature with an unreviewed text"
    run: [bun, test, tools/nv/test/site-review.test.ts]
    timeout: 900
    tests:
      - every about.md, details.md, help card, handwritten page, published program comment and the changes file is a user text
      - a program's text is its comments without directive lines, so a code edit owes no review
      - a changed text with no review stamp is named, and a stamp clears it
      - a text on the baseline is excused at its baseline hash and owes a review once it changes
      - a review stamp takes the text off the baseline, and the baseline is never written twice
      - the brief prints each text with the three questions and the prose rules
      - a feature with an unreviewed text owes Reviewed
  - id: review-green
    stage: 4
    name: "no user text has changed without a review stamp"
    run: [bun, nv, site, --check, review]
    timeout: 900
  - id: ci-check
    stage: 5
    name: "the pre-push checks are green with the changes and review parts in the website stale guard"
    run: [bun, nv, ci, --check]
    timeout: 1800
---

# every change to Novis reaches the website: stale on a behaviour change, listed on What changed, and read by a newcomer

## Why

Every addition, change and removal in Novis must reach the website, with good prose, good examples and
a good explanation. Most of the machinery exists. The generated reference renders a page for each of
the roster's 1,598 features from its `about.md`, examples and help. A handwritten page lists its
`covers:` ids and goes stale through `website/site.lock.json` (`rule:testing/website-pages-are-stamped`).
`rule:testing/feature-proofs` makes a feature owe its description and its help. Three gaps are left:

- **A behaviour change with no doc change passes.** The site hash covers the help card, `about.md` and
  the examples, not the tests. A change to what a feature does lands with a changed test, and nothing
  asks for the description, the help or the pages to be reread.
- **No change is listed for users.** `website/src/content/docs/in-depth/what-changed.mdx` promises
  highlights, new features and breaking changes per release, and has no entry. `CHANGELOG.md` is
  rendered from commit subjects, which are written for contributors.
- **Nothing checks that the text is good.** `bun nv proofs --comments` and `--check prose` count words
  and dashes. Nobody reads a text the way a newcomer does before it ships.

This goal closes all three. A feature's covering tests join its hash, and its `about.md` and help card
become stamped texts. `website/src/data/changes.json` holds what changed since the last release, and a
check fails on a feature that moved with no entry. A new feature proof, **Reviewed**, is owed by every
text a user reads: a `reader` subagent reads it as a newcomer in each goal's gate stage, and a review
stamp records it. `bun nv` checks the stamps alone, so the project still works without looper, and a
person can do the review.

The next three goals on the chain (`object-collections-fast`, `keyed-iteration`, `revocable-objects`)
are the first to owe entries and reviews under it. This goal has no gate on another goal.

## Standing decisions

**Settled with the person (do not reopen).** The hash that decides staleness gains every test whose
`covers:` marker names the feature. The feature's own `about.md` and help card become stamped texts.
A test-only edit, such as a rename, also makes them stale, and there is no exception list for it.
`changes.json` holds the unreleased entries, `bun nv site --check changes` fails on a moved feature
with no entry, and a release moves the entries onto What changed. Reviewed is a feature proof, a
`reader` subagent does the review in each gate stage with the three questions below, and existing texts
go on a baseline that never grows. Declined, each recorded in the new rule: a periodic review pass, a
check of the wording by word lists, and a changelog generated from commit messages.

**Where the code lives (my call).** A new module `tools/nv/lib/texts.ts` holds what `site.ts` and
`proofs/collect.ts` both need: `card` and `section` (moved from `tools/nv/cmd/site.ts:407` and `:431`),
the test text of a feature, the hashes below, the list of user texts and the review stamps. `site.ts`
already imports `proofs/collect.ts`, so the shared code cannot live in `site.ts` without an import
cycle. Every function takes `root`, as `featureHash` does today, so the tests run on a scratch tree.

**The four hashes (my call; safe default if one cannot be kept apart: the page hash for all of them).**

| Hash | Over | Decides |
|---|---|---|
| page hash, `featureHash` | the card, `about.md`, every file of the example directory, and the feature's tests | a handwritten page's stamp |
| text hash, today's `featureHash` | the card, `about.md` and the example directory | the search questions (`questionSources`, `tools/nv/cmd/site.ts:610`) and the baseline's restamp |
| an `about.md`'s or help card's stamp | the page hash without that text itself | whether the text is stale |
| behaviour hash | the feature's tests alone | `--check changes` |

- The questions keep the text hash, so a test edit leaves them alone: they are search phrases, and the
  person's acceptance covered the description, the help and the pages.
- An `about.md` stamp leaves out `about.md`, and a help stamp leaves out the card, so rewriting the text
  never makes the same text stale. Rewriting `about.md` still makes every covering page stale.
- The behaviour hash is the tests alone, so a rewritten description or example owes no What changed
  entry. Every kind owes at least one test (`rule:testing/feature-proofs`), so a finished feature has
  one to move.

**A feature's tests (my call).** The tests whose `covers:` marker names the feature, as `markersIn`
finds them (`tools/nv/proofs/markers.ts:22`) under `MARKER_ROOTS`. A `.nvst` case is its whole file. A
Rust `#[test]` and a package's `#[Test]` method run from the marker's line to the closing brace at the
indent of their `fn` or `function` line, so an edit to another test in the same file moves nothing. A
case credited only because it calls `Class::member(` is not in the hash: it usually tests something else.

**The lock keys (my call).** `website/site.lock.json` keeps its shape, `key -> { feature id -> hash }`.
An `about.md` is keyed by its path (`docs/examples/core/Str/length/about.md`), and a help card by
`help:<feature id>`. A language feature has no help key, because `card` gives `""` for it and its
reference section is written from the files the hash already reads. A feature with no `about.md` has
no about key; the Description proof owes that file. `--stamp` takes all three kinds of key.

**The first stamps (my call).** `bun nv site --stamp --baseline` runs once, at the end of stage 1. It
stamps every `about.md` and help card, and restamps every page whose stamp equals the text hash, which
is a stamp that was current before tests joined the hash. A page that was already stale stays stale. It
refuses to run when the lock holds any `about.md` or `help:` key, so it cannot quiet a later change.

**`changes.json` (my call).** `website/src/data/changes.json`:

```json
{
  "entries": [
    {
      "kind": "changed",
      "features": ["Core\\Str::slice"],
      "text": "`Core\\Str::slice` counts a negative start from the end of the string.",
      "breaking": true,
      "instead": "To start at the first character, use `0`."
    }
  ],
  "unseen": {
    "Core\\Str::length": { "hash": "0123456789abcdef", "why": "a test was renamed" }
  }
}
```

- `kind` is `new`, `changed` or `removed`. `features` lists roster ids, at least one. `text` is one to
  three sentences in the voice of AGENTS.md § *Text an end user reads*, and names each feature in
  backticks.
- `instead` is required on a `removed` entry and on one with `breaking: true`, and says what to write
  instead. `highlight: true` puts an entry under *Highlights* at release. A pure speed-up moves no
  hash and owes no entry, but may be added as a `changed` entry with `highlight: true`.
- `unseen` is my call, beyond what the person settled. It names a feature whose behaviour hash moved
  while nothing a user sees changed, such as a renamed test, with the hash it was written at and why.
  It clears the feature only while the hash is that one, and the release empties it. It is never used
  for a change a user can see.

**The snapshot (my call).** `data/proofs/released.json` is `{ "since": "v0.0.4", "features": { id ->
behaviour hash } }`, with a schema in `tools/nv/schema/proofs.ts`. `--check changes` compares it with
the roster now: a feature not in the snapshot needs a `new` entry, a snapshot feature not on the roster
a `removed` entry, and a feature whose behaviour hash moved a `changed` entry or a current `unseen`
record. It also fails on an entry that breaks the shape above, on `proseProblems` in its text, and on
an id neither side has. The snapshot is a file, not a build of the tag, because the roster comes from
`nvs meta --json` and a check cannot build an old binary.

**The first snapshot is the last release tag, `v0.0.4` (my call; safe fallback: the commit that lands
stage 2).** Releases are tagged `v<version>` (`git tag`: `v0.0.1` to `v0.0.4`). The roster at the tag
needs the tag's binary, which the release published. Stage 2 downloads it, makes a detached worktree at
the tag, checks out the current `tools/nv` into that worktree, and runs `bun nv site --changes
--snapshot --nvs <that binary>` there. If the roster cannot be read from that binary in one attempt, or
the download is refused, the snapshot is taken from the tree at the commit that lands stage 2, with
that commit's hash as `since`. `CHANGELOG.md` then lists the commits in between, and the handoff says so.

**The release step (my call).** `bun nv site --changes --release <version> --nvs <binary>` does three
things, and `.github/workflows/release.yml` runs it after *Bump the manifests and prepend the
changelog* (`:353`), with the Linux x86_64 binary from `dist/`. Its files join that job's `git add`
(`:368`).

- It writes `## <version>` at the top of the release list on `what-changed.mdx`, with up to four parts
  in this order: `### Highlights`, `### New features`, `### Changes`, `### Changes that can break your
  code`. Each entry is one list item, its `text` and then its `instead`. A `removed` entry and a
  breaking one go in the last part.
- It adds `{ version, change: text }` to `website/src/data/core-changelog.json` for each feature the
  entry names, in the key `changesFor` reads (`website/src/lib/feature.ts:27`).
- It empties `entries` and `unseen`, and writes a new snapshot with `since: "v<version>"`.

**The page (my call).** What changed shows the entries not yet released, under `## Not released yet`
above the newest version, from a component `website/src/components/UnreleasedChanges.astro`. The
website deploys from `main`, and the reference already shows features not yet released. The
generated reference page for a non-`Core` feature (`website/src/pages/reference/[area]/[...slug].astro`)
shows the same changelog section the `Core` member page shows
(`website/src/pages/reference/core/[...slug].astro:110`).

The wording to use, changed only where the page's other prose requires it:

- `what-changed.mdx:12` becomes: "This page lists the changes in each release of Novis, newest first.
  Each release has up to four parts: the highlights, the new features, the other changes, and the
  changes that can break your code. The changes that are not released yet are at the top."
- The component prints each entry as one list item: its `text`, then its `instead`. It prints nothing
  when `entries` is empty, and the heading says "Not released yet".

**The user texts (my call).** Each has a key and a hash of its text:

| Text | Key | Hash over |
|---|---|---|
| `about.md`, `details.md` | its path | the file |
| help card | `help:<feature id>` | `card` |
| handwritten page under `website/src/content/docs/` | its path | the file |
| a `.nvs` under `docs/examples/`, `tests/hostile/`, `benches/members/`, `website/snippets/` | its path | its `//` comment lines, without directive lines (`COMMENT_DIRECTIVE_RE`, `tools/nv/proofs/collect.ts:164`) and without a `// hostile: comment-payload` block; a file with no comment is no text |
| `website/src/data/changes.json` | its path | every entry's `text` and `instead` |

The legal pages are out, as `--check prose` leaves them out. A diagnostic's message and the help text
of an `nvs` command or flag stay out of this goal: neither is a roster feature with a text the tool can
key, and AGENTS.md holds them at the write. A later goal may add them.

**The review stamps (my call).** `data/proofs/reviewed.json` maps a key to the hash it was reviewed at.
`data/proofs/review-baseline.json` maps each text that existed when stage 4 landed to its hash then.
Both have schemas in `tools/nv/schema/proofs.ts`. A text is reviewed when its hash is its stamp's, or
is its baseline hash. `--review --stamp` writes the stamp and deletes the key from the baseline, so the
baseline only shrinks. `--review --baseline` writes the baseline once and refuses when the file exists.
The two files cost about 1 MB in the repository at first, about 12,000 texts, and nothing at run time.

**The commands (my call).**

- `bun nv site --review` lists every text that is not reviewed, one key per line, grouped by feature
  where it has one.
- `bun nv site --review --brief <key>...` prints, for a reader, the rules of `docs/agent/prose.md`, the
  three questions, and each text with its key. A help card is printed as `nvs agent show` prints it.
- `bun nv site --review --stamp <key>...` records each text's hash now. It is run by whoever fixed
  every finding, never to quiet the check.
- `bun nv site --check review` fails on every text that is not reviewed. `review` and `changes` join
  `PARTS` (`tools/nv/cmd/site.ts:339`), the website stale guard in `tools/nv/cmd/ci.ts:350`, and CI's
  proofs job (`.github/workflows/ci.yml:608`).

**Reviewed, the proof (my call).** `reviewed` joins `PROOFS` (`tools/nv/proofs/collect.ts:26`). It is
off in `POLICY` and switched on for every kind by `owes.all` in `data/proofs/policy.json`, as `about`
is. A feature owes it while its `about.md`, its `details.md`, its help card, or the comments of its
examples, attacks and bench are not reviewed. No skip file excuses it, as none excuses `about`.

**The review (my call: the three questions).** In each gate stage, the session runs `--review`, sends
`--review --brief` to `reader` subagents, at most 20 texts per reader and several readers at once, and
tells each to read no file and answer from the brief alone. For each text the reader reports:

1. each sentence it had to read twice, quoted;
2. each word or term it did not know;
3. in its own words, in two or three sentences, what the feature does, or what the page teaches.

The session compares (3) with the feature's `about.md`, or with what the page covers. Every finding is
fixed, a wrong (3) by rewriting the text that misled the reader, and then `--review --stamp` records the
texts. A reader's finding is a pointer to check, not a verdict. The procedure's home is a new section of
`docs/agent/prose.md`, *A newcomer reads it*.

**What it costs.** Nothing at run time and nothing per request. `bun nv site --check stale` scans the
`covers:` markers once per run, as `bun nv proofs` does. A test-only edit makes the feature's
`about.md`, help card and covering pages stale, and owes an `unseen` record. Each gate stage spends the
reader tokens of the texts its goal changed. The language and the `Core` API do not change.

**The rule.** `docs/rules/testing/every-change-reaches-the-website.md` is the home of gaps 1 and 2:
the stamped `about.md` and help card, `changes.json`, the snapshot and the release step. Its record
is `designed` with `"goal": "website-follows-changes"` until stage 5 flips it to `shipped`.
`rule:testing/website-pages-are-stamped` keeps the page hash, and its sentence on what the hash covers
gains the tests. `rule:testing/feature-proofs` gains Reviewed. Its **Declined** list: a periodic review
pass (it finds drift late, and never says which change caused it); a check of the wording by word lists
(AGENTS.md says nothing checks the wording, and a word check was declined before); a changelog
generated from commit messages (a commit subject is written for contributors, not for a user).

**This goal owes the website** the What changed page and its component. `bun nv` is a contributor's
tool and no roster feature, so the goal owes no feature proof and no entry of its own.

**Lanes.** Stages 1, 2 and 4 share `tools/nv/cmd/site.ts` and `tools/nv/lib/texts.ts`, so they run in
that order. Stage 3 writes `website/src/data/changes.json` only, and may run as a side goal beside stage
4 once stage 2 is done. Stage 5 touches `AGENTS.md` and `docs/agent/` only, and may run as a side goal
at any time after stage 1: every command it names is settled above.

**Limits: what stops this goal.** One attempt per item, and at most two sessions per stage. An item
still red after its attempt goes into the handoff with its cause, and the stage moves on to its other
items. A stage still open after two sessions stops the loop with `BLOCKED` naming the item. The counts
decide: every feature `--check changes` names gets an entry or an `unseen` record, and every text
`--check review` names gets its review, with no sampling. Never loosen a check to pass: no
`unseen` record for a change a user can see, no stamp without the reread or the review, no entry
deleted, and no part dropped from the stale guard.

## Stages

### 1. a change to a feature's tests makes its pages, its about.md and its help stale

**Does:** writes the rule, moves the hash code into `tools/nv/lib/texts.ts`, adds the tests to the page
hash, stamps every `about.md` and help card, and takes the first stamps.

- [ ] `docs/rules/testing/every-change-reaches-the-website.md:1`: the new rule from § *Standing
  decisions* (*The rule*, *The four hashes*, *The lock keys*, *`changes.json`*, *The snapshot*, *The
  release step*), with its reasons, its cost line and its Declined list.
  `data/rules/testing/every-change-reaches-the-website.json:1` as `designed` with its `goal`, the id
  added to `data/rules/testing.json:85`, then `bun nv rules --render` and `bun nv rules --check`.
  `rule:testing/every-change-reaches-the-website`
- [ ] `docs/rules/testing/website-pages-are-stamped.md:1`: the hash covers the feature's tests too, and
  the rule points to the new one for the `about.md` and help stamps.
  `rule:testing/website-pages-are-stamped`
- [ ] `tools/nv/lib/texts.ts:1`: `card` and `section` move here from `tools/nv/cmd/site.ts:407`, with
  `testsOf(root, entry)` over `markersIn` (`tools/nv/proofs/markers.ts:22`) and each test's text as
  § *A feature's tests* says, then the page, text, `about.md`, help and behaviour hashes.
  `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/cmd/site.ts:462`: `featureHash` is the page hash; `questionSources` (`:610`) uses the
  text hash. `staleness` (`:506`) names a stale or never-stamped `about.md` and help card beside the
  pages, and `stamp` (`:537`) takes their keys. The module doc (`:240`) and `summary` (`:329`) say so.
  `rule:testing/website-pages-are-stamped`
- [ ] `tools/nv/cmd/site.ts:2608`: `--stamp --baseline` as § *The first stamps* says.
  `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/test/site.test.ts:32`: the six tests the `feature-hash` check names, then the check run
  once by name.
- [ ] `bun nv site --stamp --baseline` once, then the `stale-green` check. A page stale before this
  goal is reread and stamped as the rule says, never left for the baseline.

### 2. an added, removed or changed feature needs an entry in changes.json, and a release moves the entries onto What changed

**Does:** adds `changes.json`, the snapshot and `--check changes`, takes the first snapshot, adds the
release step, and shows the unreleased entries on What changed.

**Split:** the session writes the code. One writing subagent writes
`website/src/components/UnreleasedChanges.astro` and the changelog section of the generic reference
page from § *The page*, once the data file's shape is on disk.

- [ ] `tools/nv/schema/proofs.ts:86`: the `released` record for `data/proofs/released.json`, beside
  `helpBacklog`. `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/cmd/site.ts:2453`: the `changes` part in `PARTS` (`:339`) and `check`, as § *The
  snapshot* says, reading `website/src/data/changes.json:1` (created with empty `entries` and `unseen`).
  `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/cmd/site.ts:2608`: `--changes --snapshot` and `--changes --release <version>`, as
  § *The snapshot* and § *The release step* say. The module doc (`:1`) lists both.
  `rule:testing/every-change-reaches-the-website`
- [ ] `.github/workflows/release.yml:353`: a step after *Bump the manifests* that unpacks the Linux
  x86_64 archive from `dist/` and runs `bun nv site --changes --release "$VERSION" --nvs <it>`. The
  commit step's `git add` (`:368`) adds `what-changed.mdx`, `changes.json`, `core-changelog.json` and
  `data/proofs/released.json`. `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/cmd/ci.ts:350` and `.github/workflows/ci.yml:608`: the website stale guard runs
  `changes` too. The comment above the CI step says what it checks.
  `rule:testing/every-change-reaches-the-website`
- [ ] `website/src/content/docs/in-depth/what-changed.mdx:12`: the sentence of § *The page*, and
  `<UnreleasedChanges />` under `## Not released yet`. `website/src/components/UnreleasedChanges.astro:1`
  and `website/src/pages/reference/[area]/[...slug].astro:90` (the changelog section), from the subagent.
  `rule:testing/every-change-reaches-the-website`
- [ ] `tools/nv/test/site-changes.test.ts:1`: the ten tests the `changes-tool` check names, then the
  check run once by name.
- [ ] The first snapshot, `data/proofs/released.json:1`, as § *The first snapshot* says. `gh release
  download v0.0.4` of this host's archive into `.agent-tmp/v0.0.4/`, the worktree under
  `.agent-tmp/worktrees/v0.0.4`, removed afterwards without a forced form. `since` is written into
  `### State`.

### 3. every feature changed since the last release has its entry

**Does:** writes an entry for every feature `--check changes` names. Every `text` and `instead` is text
an end user reads (AGENTS.md § *Text an end user reads*).

**Split:** when the check names more than 20 features, writing subagents draft the entries of one area
each (`core`, `lang`, `tools`, `config`, the rest) into `.agent-tmp/changes-<area>.json`, from what the
check prints and the features' `about.md` and tests. The session reads each draft against the tests,
merges them into the data file, and deletes the drafts.

- [ ] `bun nv site --check changes` once, and its list written into `### State`.
- [ ] `website/src/data/changes.json:1`: one entry per change a user can see, which may name several
  features. An `unseen` record only for a moved hash with nothing a user sees, with its `why`.
  `rule:testing/every-change-reaches-the-website`
- [ ] The `changes-green` check, run once.

### 4. a changed user text needs a review stamp, and Reviewed is a feature proof

**Does:** lists every user text with its key and hash, adds the review stamps, the baseline, the
commands and `--check review`, and makes Reviewed a feature proof.

- [ ] `tools/nv/lib/texts.ts:1`: `userTexts(root, world)`, each text's key and hash as § *The user
  texts* says, and the stamp and baseline reads. `rule:testing/feature-proofs`
- [ ] `tools/nv/schema/proofs.ts:86`: the `reviewed` and `review_baseline` records for
  `data/proofs/reviewed.json` and `data/proofs/review-baseline.json`. `rule:testing/feature-proofs`
- [ ] `tools/nv/cmd/site.ts:2608`: `--review`, `--review --brief`, `--review --stamp` and
  `--review --baseline`, and the `review` part in `PARTS` (`:339`) and `check` (`:2453`), as § *The
  commands* says. The module doc (`:1`) lists them. `rule:testing/feature-proofs`
- [ ] `tools/nv/proofs/collect.ts:26`: `reviewed` in `PROOFS`, `Owes` (`:30`) and `POLICY` (`:49`),
  and `owed` (`:573`) names the first unreviewed text. `tools/nv/cmd/proofs.ts:111`: its column
  heading. `data/proofs/policy.json:9`: `"reviewed": true` under `all`. The `proofPolicy` schema
  (`tools/nv/schema/proofs.ts:13`) gains the field. `rule:testing/feature-proofs`
- [ ] `docs/rules/testing/feature-proofs.md:15`: **Reviewed** after **Help**: what it proves, the
  three questions, that a reader's finding is checked, the baseline that never grows, and that no skip
  excuses it. Its record's `guardedBy` (`data/rules/testing/feature-proofs.json:1`) gains
  `tools/nv/lib/texts.ts` and `data/proofs/review-baseline.json`. `rule:testing/feature-proofs`
- [ ] `tools/nv/cmd/ci.ts:350` and `.github/workflows/ci.yml:608`: the website stale guard runs
  `review` too. `rule:testing/feature-proofs`
- [ ] `tools/nv/test/site-review.test.ts:1`: the seven tests the `review-tool` check names, then the
  check run once by name.
- [ ] `bun nv site --review --baseline` once, then the `review-green` check.

### 5. every goal names what it owes the website, and its gate stage runs the review

**Does:** writes the procedure where every session reads it, and flips the rule to `shipped`.

- [ ] `docs/agent/prose.md:18`: a section *A newcomer reads it*: the commands in order, the batches,
  the reader's prompt (read no file; the three questions), comparing (3), fixing every finding, and
  the stamp. `rule:testing/feature-proofs`
- [ ] `docs/agent/conventions.md:648`: two bullets in § *A goal*. Every goal's last content stage
  names the website pages it rereads and stamps and the `changes.json` entries it writes, by kind and
  feature. Every gate stage starts with the review of `docs/agent/prose.md` § *A newcomer reads it*,
  before its first `looper verify`. `rule:testing/every-change-reaches-the-website`
- [ ] `docs/agent/loop-authoring.md:194`: one line under § 6 that the last content stage owes the
  website and the gate stage runs the review, pointing to `conventions.md` § *A goal*.
  `rule:testing/every-change-reaches-the-website`
- [ ] `AGENTS.md:91`: one bullet under § *The rules only this project has*, after the feature proofs
  one: a change reaches the website, so `bun nv site --check stale`, `changes` and `review` are green
  before a goal ends, and the gate stage runs the newcomer review.
  `rule:testing/every-change-reaches-the-website`
- [ ] `docs/agent/commands.md:470`: the stamp, changes and review commands in the website block.
  `rule:testing/every-change-reaches-the-website`
- [ ] `data/rules/testing/every-change-reaches-the-website.json:1`: `shipped`, its `goal` removed, and
  its `guardedBy` lists `tools/nv/lib/texts.ts`, `tools/nv/test/site.test.ts` and
  `tools/nv/test/site-changes.test.ts`. `bun nv rules --render`.
  `rule:testing/every-change-reaches-the-website`
- [ ] The `ci-check` check, run once, and each red fixed.

### 6. the goal is verified once, and every failure is fixed

- [ ] The review of `docs/agent/prose.md` § *A newcomer reads it* over every text `bun nv site
  --review` lists. This goal's own `what-changed.mdx` and `changes.json` are among them.
- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person. Their calls: every addition, change or
removal must reach the website with good prose, good examples and a good explanation, as a new rule and
a new proof; the plan of three gaps and three fixes in § *Why* was approved. The authoring session's
calls: the exact data file names and shapes, the first snapshot at `v0.0.4` with the landing commit as
the fallback, accepting the staleness from a test-only edit, the reader's three questions, the order
of the four goals, and every other choice in § *Standing decisions*. Nothing is built yet.

### Next

Stage 1, from its first item: the rule fragment `docs/rules/testing/every-change-reaches-the-website.md:1`,
then `tools/nv/lib/texts.ts:1` and `tools/nv/cmd/site.ts:462`.

### Backlog

