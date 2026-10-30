---
looper: 1
title: "the tree tells the truth before the release: no stale rule, no hidden gap, nothing private"
summary: "bun nv owners counts every Not-on-disk section as a gap, no rule fragment claims something exists or is missing that is not so, every internal crate is publish = false, and the checks below pass."
context: [tools/nv/cmd/owners.ts, tools/nv/schema/gap.ts]
stages:
  - id: 1
    title: "bun nv owners refuses a Not-on-disk paragraph that cites no gap record"
    status: done
    context: [tools/nv/test/owners-rules.test.ts, docs/rules/testing/a-failing-proof-is-fixed-or-recorded.md, docs/agent/conventions.md]
  - id: 2
    title: "every packaging and security fragment says what is on disk, and each gap it names is a record"
    status: done
    context: [docs/rules/packaging/package-toml-is-a-tool-input.md, crates/nvs-cli/src/package/mod.rs, docs/agent/doc-style.md]
  - id: 3
    title: "every core-classes, ide and testing fragment says what is on disk, and each gap it names is a record"
    status: done
    context: [docs/rules/core-classes/spreadsheet-roster.md, extensions/spreadsheet/src/lib.rs, crates/nvs-cli/src/debug.rs, docs/agent/doc-style.md]
  - id: 4
    title: "no checkout path, every crate is publish = false, and the license names its scope"
    status: done
    context: [README.md, Cargo.toml, tools/nv/cmd/lints.ts, crates/nvs-cli/src/cache.rs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: owners-fragment-tests
    stage: 1
    name: bun nv owners reads every rule fragment's Not-on-disk and Not-shipped paragraph, and refuses one that cites no gap record
    run: [bun, test, tools/nv/test/owners-fragments.test.ts]
    tests:
      - a Not on disk paragraph that cites no gap record is a finding
      - a Not shipped paragraph is read the same way
      - a paragraph that cites an existing gap record is not a finding
      - a paragraph that cites a gap record that does not exist is a finding
      - unmarked running prose such as not written yet is not a finding
      - a chapter file under docs/rules is not read
      - a Not done yet list in a module doc that cites no gap record is a finding
      - a Cost paragraph after a Not done yet list is not part of the list
  - id: owners-check
    stage: 3
    name: every gap record and every Not-on-disk paragraph names an owner the chain knows
    run: [bun, nv, owners, --check]
    output_contains: ["no gap lives only in prose"]
  - id: stale-sections-gone
    stage: 3
    name: the fragments whose code is on disk carry no Not-on-disk or Not-shipped paragraph
    run: [git, grep, -nE, '^\*\*Not (on disk|shipped)\.\*\*', --, docs/rules/core-classes/passkey-verifies-a-webauthn-ceremony.md, docs/rules/core-classes/web-push-sends-an-encrypted-message.md, docs/rules/packaging/package-toml-is-a-tool-input.md, docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md, docs/rules/packaging/an-external-file-is-pinned-by-url-sha256-and-size.md, docs/rules/packaging/only-a-major-may-break-the-api.md, docs/rules/packaging/a-git-dependency-is-root-only.md, docs/rules/packaging/a-package-name-is-vendor-slash-name.md, docs/rules/packaging/a-publish-token-is-scoped-and-expires.md, docs/rules/packaging/publishing-never-overwrites.md, docs/rules/packaging/search-sorts-by-the-readers-choice.md, docs/rules/packaging/the-registry-builds-the-archive-from-git.md, docs/rules/packaging/the-registry-keeps-an-append-only-log.md, docs/rules/security/package-authority-is-granted-one-line-at-a-time.md, docs/rules/security/authority-is-the-enclosing-namespace.md, docs/rules/core-classes/process-spawn.md]
    exit: nonzero
    output_exact: ""
  - id: rules-rendered
    stage: 3
    name: every rule chapter is what its fragments render
    run: [bun, nv, rules, --render, --check]
  - id: no-checkout-path
    stage: 4
    name: no tracked file names a developer's checkout path or machine
    run: [git, grep, -nIE, '[A-Za-z]:[/\\]+mwl\b|/(mnt/)?e/mwl\b|mwltargetdebug|this Windows machine', --, ., ':!tools/nv/test', ':!docs/goals']
    exit: nonzero
    output_exact: ""
  - id: publish-false-test
    stage: 4
    name: bun nv lints refuses a crate manifest that does not inherit publish = false
    run: [bun, test, tools/nv/test/lints.test.ts]
    tests:
      - a crate manifest that does not inherit publish is a finding
  - id: lints-check
    stage: 4
    name: every crate manifest inherits the lint table and publish = false
    run: [bun, nv, lints, --check]
  - id: manifests-plain
    stage: 4
    name: no crate description names a milestone or a tier, and no module allows dead code wholesale
    run: [git, grep, -nE, '^description = .*(\(M[0-9]|Tier [0-9])|^#!\[allow\(dead_code\)\]', --, 'crates/*/Cargo.toml', 'crates/*.rs']
    exit: nonzero
    output_exact: ""
  - id: license-scope
    stage: 4
    name: the README's license section names the looper kit and the one copyleft dependency
    file: README.md
    output_contains: ["tools/looper", "MPL-2.0"]
  - id: no-permissive-claim
    stage: 4
    name: the README no longer says every third-party component is permissive
    run: [git, grep, -nF, 'Every third-party component in the `nvs` binary has a permissive license', --, README.md]
    exit: nonzero
    output_exact: ""
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, release-hygiene]
    reach: true
    floor: false
---

# the tree tells the truth before the release: no stale rule, no hidden gap, nothing private

## Why

This goal is first on the release chain, so every later goal starts from a tree that tells the truth.

- **The gap register misses most gaps.** Twenty-nine rule fragments carry a `**Not on disk.**` or
  `**Not shipped.**` paragraph, and four more say the same in running prose. `bun nv owners --check`
  reports no gap, because it reads Rust sources, manifests and conformance cases, never a rule
  fragment (`tools/nv/cmd/owners.ts:70`). So no goal owns that work, and no `--closes` gate holds it.
- **Most of those paragraphs are false.** `Core\Passkey`, `Core\WebPush`, `nvs pkg publish`,
  `audit` and `add`, the viewer's API, the per-namespace grant table and most of the registry
  service are on disk, and their rules still say they are not.
- **Three extension module docs keep `**Not done yet.**` lists** that no record backs either.
- **The tree names one developer's checkout.** Seven playbook lines, a tool comment and `AGENTS.md`
  name a drive path or "this Windows machine".
- **The release metadata is loose.** No crate says `publish = false`, two crate descriptions name
  milestones and tiers, a 3,700-line module allows dead code wholesale, and the README calls every
  dependency permissive while one is MPL-2.0.

## Standing decisions

**A Not-on-disk paragraph cites the gap records it stands for.** The paragraph opens with the bold
lead `**Not on disk.**`, `**Not shipped.**` or `**Not done yet.**`, and somewhere in it names each record as
`gap:<crate>/<slug>`, the record's id. Example:
`**Not on disk.** The registry refuses no git dependency yet (gap:nvs-cli/registry-refuses-a-git-dependency).`
This mirrors a proof's `// proof: gap <id>` marker (`rule:testing/a-failing-proof-is-fixed-or-recorded`).
A reader of the fragment sees which record backs it, and deleting the record when the work lands
makes the paragraph a finding, so the builder rewrites the fragment in the same slice. Recorded in
the header of `tools/nv/cmd/owners.ts` and in `docs/agent/conventions.md` § *A rule*.

**The detector reads only the bold lead, in fragments and module docs.** It reads
`docs/rules/<topic>/*.md`, never the generated chapters `docs/rules/<topic>.md`, which repeat every
fragment. It also reads the `//!` and `///` doc comments of every file the prose scan reads, plus
`extensions/*/src/**/*.rs`. A paragraph is a finding when its first line starts with one of the three
bold leads and it cites no `gap:` id, or cites one with no record. In a doc comment the paragraph
runs on through its bullet list to the first blank doc line that no bullet continues. So a `Cost:`
paragraph after a list is outside it, because a cost is a documented trade-off, not a gap. It is a new prose kind, `rule`, in `outsideBlocks`
(`tools/nv/cmd/owners.ts:215`), so `--prose` lists it and `--check` refuses it with the other prose
gaps. Running prose such as "not written yet" is never read. So the two false matches stay as
written: `docs/rules/core-api/field-wise-precedence.md:7` and
`docs/rules/core-classes/decompression-bound.md:12`. A real gap written as running prose gets a
bold-led paragraph of its own in stages 2 and 3. Declined, written into the owners.ts header:
- **A `rule` field on the gap record.** The fragment's reader could not see which record backs the
  paragraph, and a second link drifts from the first.
- **A `partial` rule status.** A shipped rule with one owed corner is still shipped, and the corner
  is a gap. A status would also move the whole rule into the designed roster.
- **A word scan over unmarked prose.** It flags field-wise-precedence and decompression-bound, and
  every such list grows an allowlist.

**A gap record's shape.** `data/gaps/<crate>/<slug>.json`, per `tools/nv/schema/gap.ts:6`:
`{"module": "<repo-rooted file or directory whose code owes it>", "title": "<one claim>", "text": "<what closes it, and the rule>", "goal": "<owner slug>"}`.
One record per paragraph. A paragraph whose items belong to two goals is split into two records,
and it cites both. `<crate>` is the crate that holds `module`. A registry record with no registry
code on disk yet uses `nvs-cli` and the module `crates/nvs-cli/src/package`, and `registry-service`
moves it when its own crate lands. A spreadsheet record uses `nvs-spreadsheet` (the package name in
`extensions/spreadsheet/Cargo.toml`). Safe fallback: if the store refuses a directory name, use the
nearest workspace crate and say so in the commit.

**Each paragraph is STALE, REAL or a BOUND, and the code decides.** Read the paragraph, then grep the
code it names. The verdicts below were checked against the code. Where the code says otherwise, the
code wins and the commit says why.
- **STALE:** the code exists. Rewrite the fragment to describe the present, in the voice of
  `docs/agent/doc-style.md`: no "now", no "yet", no history, and the code path in backticks. Usually
  the paragraph's content moves into the body and the paragraph goes.
- **REAL:** the code does not exist. The paragraph keeps its bold lead, cites its record, and the
  record names the owner below. The user decided that every real item is built before the first
  release, so no real item is reworded into a bound.
- **BOUND:** the rule states a permanent limit by design, such as `rule:core-api/tier-roster`'s
  "Ext when demand appears" or the telemetry default URL. It moves out of the bold paragraph into
  the body as a present-tense sentence, or into **Declined:** with its reason.
- **Unsure:** REAL. An owned gap is visible and a goal closes it. A wrongly deleted one is lost.

**The owners.** The packaging items still real go to `registry-service`: fetch refusing an
archive with an `nvs.toml`, an extension package packed and fetched as one, the advisory route and
range, the release confirmation pages, and the CDN purge on withhold. Socket activation, the regex
`[regex]` block, the debugger's task tree and the rename of a class-scoped type alias
(`crates/nvs-lsp/src/rename.rs:108`) go to `runtime-leftovers`. Every `core-classes/spreadsheet-*`
item goes to `spreadsheet-second-wave`, and so do the bullets at `extensions/spreadsheet/src/lib.rs:42-52`.
The spreadsheet bullets from `lib.rs:53` on, and both PDF lists, go to `extension-leftovers`. Any
other real item goes to `runtime-leftovers`. No record names `release-hygiene`. The production
verifier key (`crates/nvs-cli/src/package/manifest.rs:93`) is the user's release step, not a gap.
`rule:core-classes/process-spawn`'s paragraph is reworded with no record, because
`runtime-leftovers` tests that behaviour itself.

**Checkout paths become `<repo>`.** A Windows checkout path is `<repo>`, so `E:/…/target/debug/nvs.exe`
becomes `<repo>/target/debug/nvs.exe`. A WSL path is `/mnt/<drive>/<repo>`. The playbook bullet about
lost backslashes keeps its point with `<repo>\target\debug\nvs.exe` and `<repo>targetdebugnvs.exe`. The
sample in `tools/nv/accept/mirror.ts:53` becomes `D:\src\app` and `/mnt/d/src/app`. `AGENTS.md:92`
says "not just on your own machine". Each bullet's `until:` trailer stays as written. Sample paths in
`tools/nv/test/*.test.ts` are test inputs and stay. The check skips `docs/goals/`, whose files quote
the pattern.

**The spreadsheet style enums are REAL, not a BOUND.** `rule:enums/an-enum-lives-in-the-class-that-uses-it`
binds every built-in component's enum, so the style enums at namespace scope, and `Visibility` beside
them, break a shipped rule. Record `nvs-spreadsheet/spreadsheet-style-enums` names
`spreadsheet-second-wave`, and the roster's `Visibility` sentence moved into its Not-shipped paragraph.

**The license scope.** The root MIT `LICENSE` covers the whole repository, the looper kit in
`tools/looper/` included. `tools/looper/` is synced and never hand-edited, so the scope is stated
outside it: one sentence in README.md § *License* (`README.md:163`) and the `tools/` line of
CONTRIBUTING.md's layout (`CONTRIBUTING.md:184`). README.md:165 becomes: every third-party component
has a license `deny.toml` allows. Most are permissive. Five are MPL-2.0, whose copyleft covers only
their own files, and Novis uses them unchanged: `THIRD-PARTY-LICENSES.txt` lists `avif-parse`,
`cssparser`, `cssparser-macros`, `dtoa-short` and `selectors`, so the README names all five, not
`avif-parse` alone. The README is text an end user reads, so AGENTS.md
§ *Text an end user reads* holds.

**`publish = false` is inherited.** `[workspace.package]` (`Cargo.toml:36`) gains `publish = false`,
and every crate under `crates/` writes `publish.workspace = true`. `crates/nvs-repo/Cargo.toml:10`'s
own `publish = false` becomes the inherited form. Nothing is published to crates.io, and the comment
above `[workspace.package]` says so. `bun nv lints --check` refuses a crate manifest without it,
because `tools/nv/cmd/lints.ts` already owns what each crate manifest inherits. Safe fallback: if
Cargo refuses `publish.workspace`, each crate writes `publish = false` and the check accepts that
form. The `extensions/*` manifests are their own workspaces and already say `publish = false`.
The two bench crates, `benches/abi-probe` and `benches/serve-probe`, are workspace members too, so
they write the inherited form as well, and the check covers every member with one rule rather than
a `crates/` special case. The check also refuses a root `[workspace.package]` without the key.

**Crate descriptions say what the crate does.** `crates/nvs-cli/Cargo.toml:3`: "The nvs
command-line binary: run, test, serve, check, format and the other commands". `crates/nvs-stdlib/Cargo.toml:3`:
"The Novis standard library: the `Core` namespace's native Rust members and the signature registry
the compiler resolves them through". No milestone and no tier.

**`cache.rs` allows no dead code wholesale.** Delete `#![allow(dead_code)]`
(`crates/nvs-cli/src/cache.rs:199`) and its comment (`:194`), then build. An item only the module's
tests reach gets `#[cfg(test)]`. An item nothing reaches is deleted. Safe fallback, for an item a
test needs that cannot be gated without splitting an `impl` or a `match`: an item-level
`#[cfg_attr(not(test), expect(dead_code, reason = "…"))]`.

**`a.out` is local clutter.** It is untracked and ignored (`.gitignore:112`). Delete the file. The
ignore line stays.

**Trade-offs.** Security: none on the request path. A gap that only prose knew is now held by a
goal's `--closes` gate. Latency and memory: no runtime change. Simplicity: one register for owed
work, and the rule fragments say only what is true.

**Lanes.** Stage 1 lands the citation format the next two stages write. Stages 2 and 3 edit
disjoint fragments and gap records, but both run `bun nv rules --render`, which rewrites
`docs/ground-rules.md` too, so they run in order. Stage 4 touches no file of stages 1 to 3, so
it may run as a side goal beside them. Between stage 1 and the end of stage 3, `bun nv owners
--check` is red by design: its `--prose` list is the worklist.

## Stages

### 1. bun nv owners refuses a Not-on-disk paragraph that cites no gap record

**Does:** the owners gate reads every bold not-built paragraph in a rule fragment and in a Rust
module doc, and refuses one that cites no existing gap record.

- [x] `rule:testing/a-failing-proof-is-fixed-or-recorded`: a new `ProseKind` `rule`, its `PROSE_WHY` line, and a function that reads `docs/rules/<topic>/*.md` under a `root` argument, finds each paragraph whose first line starts with a bold lead, and reports it when it cites no `gap:<id>` or cites one no record has. `outsideBlocks` calls it. `tools/nv/cmd/owners.ts:70`, `tools/nv/cmd/owners.ts:197`, `tools/nv/cmd/owners.ts:201`, `tools/nv/cmd/owners.ts:215`
- [x] `rule:testing/a-failing-proof-is-fixed-or-recorded`: the same check over `//!` and `///` doc comments. `PROSE_SOURCES` gains `extensions/*/src/**/*.rs`, and a doc line starting with a bold lead opens a block. The block runs to the first blank doc line that no bullet continues, and it must cite at least one existing `gap:<id>`. Today that finds `extensions/spreadsheet/src/lib.rs:38`, `extensions/pdf/src/lib.rs:45` and `extensions/pdf/src/layout/table.rs:62`. `tools/nv/cmd/owners.ts:70`, `tools/nv/cmd/owners.ts:215`
- [x] `rule:testing/a-failing-proof-is-fixed-or-recorded`: the header says the prose scan reads rule fragments, the citation format, and the three Declined bullets of the standing decision. `tools/nv/cmd/owners.ts:34`
- [x] `rule:testing/a-failing-proof-is-fixed-or-recorded`: the tests the `owners-fragment-tests` check names, over a temporary root as `owners-rules.test.ts` builds one. Run once by name. `tools/nv/test/owners-fragments.test.ts:1`, `tools/nv/test/owners-rules.test.ts:37`
- [x] `rule:testing/a-failing-proof-is-fixed-or-recorded`: conventions.md § *A rule* gains one paragraph: what is not built is a bold-led paragraph that cites its gap records, and `bun nv owners --check` refuses one that does not. `docs/agent/conventions.md:458`
- [x] Run `bun nv owners --prose` once and paste its `rule` count into the handoff. It should list the 29 fragment paragraphs and the 3 module-doc lists that stages 2 and 3 take. `tools/nv/cmd/owners.ts:250`

### 2. every packaging and security fragment says what is on disk, and each gap it names is a record

**Does:** every packaging and security fragment's Not-on-disk paragraph is rewritten to the present
or cites a gap record, and every real item there has an owner.

**Split:** none needed. Each verdict below was checked against the code when the goal was written.
The session confirms each one with the anchor's grep as it rewrites the fragment.

Fully STALE: the code is on disk. Rewrite the fragment to the present and drop the paragraph, with no record:
- [x] `rule:packaging/package-toml-is-a-tool-input`: `audit`, `publish`, `retract`, `unlist`, `login` and `whoami` exist, and the editor reads `[grants]`. `docs/rules/packaging/package-toml-is-a-tool-input.md:53`, `crates/nvs-cli/src/package/mod.rs:369`, `crates/nvs-lsp/src/grants.rs:1`
- [x] `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`: `nvs pkg publish` and `--dry-run` exist. `docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:95`, `crates/nvs-cli/src/package/publish.rs:18`
- [x] `rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`: `publish` exists, and the package page lists each external file with its host, size, hash and license. `docs/rules/packaging/an-external-file-is-pinned-by-url-sha256-and-size.md:70`, `apps/registry/src/PackagePage.nvs:414`
- [x] `rule:packaging/the-first-party-components-are-built-in`: check the PDF exports and the spreadsheet reader and template against "return `runtime`", and describe what each export does. This paragraph has no bold lead. `docs/rules/packaging/the-first-party-components-are-built-in.md:53`, `extensions/pdf/src/lib.rs:1`, `extensions/spreadsheet/src/lib.rs:914`
- [x] `rule:packaging/only-a-major-may-break-the-api`: `nvs pkg api` extracts a package's API, and `nvs pkg diff` classifies each change and refuses a breaking minor. `docs/rules/packaging/only-a-major-may-break-the-api.md:56`, `crates/nvs-cli/src/package/api.rs:1`, `crates/nvs-cli/src/package/diff.rs:50`
- [x] `rule:security/package-authority-is-granted-one-line-at-a-time`: `nvs pkg add` prints every capability the new subgraph requests. `docs/rules/security/package-authority-is-granted-one-line-at-a-time.md:43`, `crates/nvs-cli/src/package/minimum.rs:251`, `crates/nvs-cli/src/package/report.rs:5`
- [x] `rule:packaging/a-git-dependency-is-root-only`: the publish build refuses a git dependency. `docs/rules/packaging/a-git-dependency-is-root-only.md:34`, `crates/nvs-cli/src/package/build.rs:367`
- [x] `rule:packaging/a-package-name-is-vendor-slash-name`: the registry refuses a name that differs from a taken one only by case, and checks the `Novis\` and `Core\` vendors against the project account. `docs/rules/packaging/a-package-name-is-vendor-slash-name.md:42`, `apps/registry/src/Publish.nvs:121`
- [x] `rule:packaging/a-publish-token-is-scoped-and-expires`: creating and revoking a token writes a log entry and notifies the owners. `docs/rules/packaging/a-publish-token-is-scoped-and-expires.md:30`, `apps/registry/src/Events.nvs:74`
- [x] `rule:packaging/publishing-never-overwrites`: the registry refuses a publish from an account with no verified notification channel. `docs/rules/packaging/publishing-never-overwrites.md:48`, `apps/registry/src/App.nvs:989`, `apps/registry/src/Notices.nvs:107`
- [x] `rule:packaging/search-sorts-by-the-readers-choice`: the registry's search and ranking exist. `docs/rules/packaging/search-sorts-by-the-readers-choice.md:67`, `apps/registry/src/Search.nvs:1`, `apps/registry/src/Ranking.nvs:1`
- [x] `rule:packaging/the-registry-builds-the-archive-from-git`: the repository route needs a recent passkey sign-in, and writes a `repository` log entry. `docs/rules/packaging/the-registry-builds-the-archive-from-git.md:43`, `apps/registry/src/App.nvs:789`, `apps/registry/src/Events.nvs:64`
- [x] `rule:packaging/the-registry-keeps-an-append-only-log`: the service exists, and `nvs pkg audit` compares the second copy of the checkpoint. The verifier key is the user's own release step, so it gets no record. Write it as a present fact: the default registry is read once its key is in the binary (`DEFAULT_KEY`). `docs/rules/packaging/the-registry-keeps-an-append-only-log.md:96`, `crates/nvs-cli/src/package/audit.rs:12`, `crates/nvs-cli/src/package/manifest.rs:93`

Partly STALE. Rewrite the stale part, and cite a `registry-service` record for the rest:
- [x] `rule:packaging/a-known-bad-version-is-retracted-not-deleted`: `nvs pkg audit` exists. The advisory route and its version range are real, at both paragraphs: record `nvs-cli/advisory-route-and-range`. `docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:41`, `docs/rules/packaging/a-known-bad-version-is-retracted-not-deleted.md:55`, `crates/nvs-cli/src/package/audit.rs:1`, `data/gaps/nvs-cli/advisory-route-and-range.json:1`
- [x] `rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`: the pending feed, the owners' notice and the pending list in `publish` and `whoami` exist. The confirmation pages, where an owner confirms with a passkey in the browser, are real: record `nvs-cli/release-confirmation-pages`. The advisory route cites `nvs-cli/advisory-route-and-range`. `docs/rules/packaging/a-release-waits-seven-days-unless-an-owner-confirms.md:44`, `apps/registry/src/Feeds.nvs:85`, `crates/nvs-cli/src/package/login.rs:19`, `data/gaps/nvs-cli/release-confirmation-pages.json:1`
- [x] `rule:packaging/the-operator-only-withholds`: `audit` exists. The CDN purge is real: record `nvs-cli/withhold-purges-the-cdn`. `docs/rules/packaging/the-operator-only-withholds.md:36`, `data/gaps/nvs-cli/withhold-purges-the-cdn.json:1`

REAL, owned by `registry-service`. The paragraph cites its record:
- [x] `rule:packaging/a-package-cannot-reach-its-host`: `fetch` does not refuse an archive that holds an `nvs.toml`. Record `nvs-cli/fetch-refuses-an-archive-with-nvs-toml`. `docs/rules/packaging/a-package-cannot-reach-its-host.md:20`, `crates/nvs-cli/src/package/fetch.rs:1`, `data/gaps/nvs-cli/fetch-refuses-an-archive-with-nvs-toml.json:1`
- [x] `rule:packaging/a-package-is-its-digest`: an extension package is not packed or fetched as one. Record `nvs-cli/extension-package-is-packed-and-fetched`. `docs/rules/packaging/a-package-is-its-digest.md:74`, `crates/nvs-cli/src/package/pack.rs:1`, `data/gaps/nvs-cli/extension-package-is-packed-and-fetched.json:1`

REAL, owned by `runtime-leftovers`, written as running prose today. It gets a bold-led paragraph:
- [x] `rule:packaging/the-generated-unit-is-hardened`: socket activation, record `nvs-cli/socket-activation` with module `crates/nvs-cli/src/service.rs`. `docs/rules/packaging/the-generated-unit-is-hardened.md:77`, `data/gaps/nvs-cli/socket-activation.json:1`

STALE in security:
- [x] `rule:security/authority-is-the-enclosing-namespace`: the type checker reads `[grants]` per namespace, and the `grants-*` conformance cases guard it. Rewrite and drop the paragraph. `docs/rules/security/authority-is-the-enclosing-namespace.md:16`, `crates/nvs-types/src/capability.rs:235`, `tests/conformance/cap/grants-a-namespace-holds-only-what-it-is-granted.nvst:1`

Then:
- [x] `bun nv rules --render`, and `bun nv owners --prose` lists no `rule` finding under `docs/rules/packaging/` or `docs/rules/security/`. `tools/nv/cmd/rules.ts:5`

### 3. every core-classes, ide and testing fragment says what is on disk, and each gap it names is a record

**Does:** the remaining fragments' Not-on-disk paragraphs are rewritten to the present or cite a gap
record, and the editor's one "not available yet" message has a record.

**Split:** the seven spreadsheet fragments go to one `reader` subagent first, which returns each
item's verdict (REAL or BOUND) with evidence. The session writes every fragment and record.

STALE. Rewrite and drop the paragraph:
- [x] `rule:core-classes/passkey-verifies-a-webauthn-ceremony`: `docs/rules/core-classes/passkey-verifies-a-webauthn-ceremony.md:51`, `crates/nvs-stdlib/src/passkey.rs:1`
- [x] `rule:core-classes/web-push-sends-an-encrypted-message`: `docs/rules/core-classes/web-push-sends-an-encrypted-message.md:46`, `crates/nvs-stdlib/src/webpush.rs:1`
- [x] `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`: the API, search, replay, settings and to-test hand-off exist. Any view the code lacks (grouping, compare) is REAL, record `nvs-cli/viewer-views` owned by `runtime-leftovers`. `docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md:113`, `crates/nvs-cli/src/debug/api.rs:1`, `crates/nvs-cli/src/debug/search.rs:1`, `crates/nvs-cli/src/debug/replay.rs:1`, `crates/nvs-cli/src/debug/settings.rs:1`, `crates/nvs-cli/src/debug/to_test.rs:1`

- [x] `rule:core-classes/process-spawn`: the paragraph states intended behaviour. Reword it as the rule's present statement without the bold lead, and write no record, because `runtime-leftovers` tests it and fixes the handle hole it finds. `docs/rules/core-classes/process-spawn.md:35`, `crates/nvs-stdlib/src/process.rs:1`

REAL, owned by `runtime-leftovers`:
- [x] `rule:core-classes/regex-compile-time-tiering`: running prose today, so it gets a bold lead. Record `nvs-config/regex-deny-block`. `docs/rules/core-classes/regex-compile-time-tiering.md:18`, `crates/nvs-config/src/lib.rs:1`, `data/gaps/nvs-config/regex-deny-block.json:1`
- [x] `rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`: the task tree, out of the bullet into a bold-led paragraph. Record `nvs-cli/debugger-task-tree`. `docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md:28`, `crates/nvs-cli/src/dap/mod.rs:1`, `data/gaps/nvs-cli/debugger-task-tree.json:1`
- [x] `rule:types/class-scoped-alias`: renaming a type alias a class declares. Record `nvs-lsp/rename-a-class-scoped-alias`. No fragment changes. `crates/nvs-lsp/src/rename.rs:108`, `data/gaps/nvs-lsp/rename-a-class-scoped-alias.json:1`

REAL or BOUND, owned by `spreadsheet-second-wave`. One record per paragraph under `data/gaps/nvs-spreadsheet/`:
- [x] `rule:core-classes/spreadsheet-evaluation-and-roster`: evaluate and writing ods, record `spreadsheet-evaluate-and-write-ods`. `docs/rules/core-classes/spreadsheet-evaluation-and-roster.md:35`
- [x] `rule:core-classes/spreadsheet-has-no-io`: other CSV dialects and encodings, record `spreadsheet-csv-dialects`. `docs/rules/core-classes/spreadsheet-has-no-io.md:74`
- [x] `rule:core-classes/spreadsheet-reads-as-a-stream`: the xlsb, xls and ods parts, ods number styles and month names, record `spreadsheet-read-parts-of-every-format`. `docs/rules/core-classes/spreadsheet-reads-as-a-stream.md:113`
- [x] `rule:core-classes/spreadsheet-roster` and `rule:core-classes/spreadsheet-sheet-end-parts`: `Cell::rich`, one record `spreadsheet-cell-rich` cited by both. `docs/rules/core-classes/spreadsheet-roster.md:50`, `docs/rules/core-classes/spreadsheet-sheet-end-parts.md:83`
- [x] `rule:core-classes/spreadsheet-styles-and-defaults`: the style enums under `Style`, record `spreadsheet-style-enums`. `docs/rules/core-classes/spreadsheet-styles-and-defaults.md:67`
- [x] `rule:core-classes/spreadsheet-template-fill`: blocks across sheets, shared formulas, `x14` formats and iterators, record `spreadsheet-template-fill-limits`. `docs/rules/core-classes/spreadsheet-template-fill.md:47`

The three `**Not done yet.**` lists in extension module docs. Each list cites its records, and a list split between two owners cites both. A `Cost:` paragraph after a list is a documented trade-off, not a gap:
- [x] `rule:core-classes/spreadsheet-roster`: the bullets at lines 42-52 (tables, template rows, template values) → record `nvs-spreadsheet/spreadsheet-unchecked-against-excel`, owned by `spreadsheet-second-wave`. `extensions/spreadsheet/src/lib.rs:38`, `extensions/spreadsheet/src/lib.rs:42`, `data/gaps/nvs-spreadsheet/spreadsheet-unchecked-against-excel.json:1`
- [x] `rule:core-classes/spreadsheet-roster`: the bullets from line 53 on (number formats, ods and merges, extras, rows, sheets, the decompression-bound message, charts, sheet-end parts and the rest) → record `nvs-spreadsheet/spreadsheet-reader-and-writer-leftovers`, owned by `extension-leftovers`. `extensions/spreadsheet/src/lib.rs:53`, `data/gaps/nvs-spreadsheet/spreadsheet-reader-and-writer-leftovers.json:1`
- [x] `rule:core-classes/pdf-roster`: every bullet of the PDF list → record `nvs-pdf/pdf-leftovers`, owned by `extension-leftovers`. `extensions/pdf/src/lib.rs:45`, `data/gaps/nvs-pdf/pdf-leftovers.json:1`
- [x] `rule:core-classes/pdf-tables-across-pages`: the table list → record `nvs-pdf/pdf-table-leftovers`, owned by `extension-leftovers`. Its `Cost:` paragraph stays as written. `extensions/pdf/src/layout/table.rs:62`, `data/gaps/nvs-pdf/pdf-table-leftovers.json:1`

Then:
- [x] `bun nv rules --render`, then `bun nv owners --check` passes and lists the records by owner. `tools/nv/cmd/rules.ts:5`, `tools/nv/cmd/owners.ts:375`

### 4. no checkout path, every crate is publish = false, and the license names its scope

**Does:** no tracked file names a developer's machine, every crate inherits `publish = false`, the
crate metadata is plain, and the README states the license scope truthfully.

Paths, one group:
- [x] No rule governs this; the standing decision *Checkout paths become `<repo>`* is its spec. Each playbook line's checkout path becomes `<repo>`, its `until:` trailer unchanged. `docs/agent/playbook.md:96`, `docs/agent/playbook.md:165`, `docs/agent/playbook/running-things/nv-reference-runs-its-own-binary-not-target-debug.md:4`, `docs/agent/playbook/running-things/a-windows-path-handed-to-npm-run-nvs-path-from-a-posix-shell.md:3`, `docs/agent/playbook/running-things/a-wsl-build-is-killed-as-a-hang-unless-it-prints.md:7`, `docs/agent/playbook/writing-a-test-case/a-case-that-tries-both-platforms-shells-and-catches-ioerror.md:1`, `docs/agent/playbook/tooling/a-heredoc-in-a-bash-call-arrives-with-each-double-backslash-as-one.md:1`
- [x] The same standing decision: the sample path in the doc comment, and "this Windows machine". `tools/nv/accept/mirror.ts:53`, `AGENTS.md:92`

License, one group:
- [x] `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`: README § *License* names the scope and `avif-parse`'s MPL-2.0, and CONTRIBUTING's `tools/` line says the root license covers the looper kit. `README.md:163`, `README.md:165`, `CONTRIBUTING.md:184`

Crates, one group:
- [x] No rule governs this; the standing decision *`publish = false` is inherited* is its spec, and the `[workspace.package]` comment its home. `publish = false` in `[workspace.package]` with its comment, and `publish.workspace = true` in all 21 crate manifests: `nvs-cli`, `nvs-codegen`, `nvs-config`, `nvs-db`, `nvs-diagnostics`, `nvs-ext`, `nvs-fmt`, `nvs-footprint`, `nvs-hir`, `nvs-host`, `nvs-ir`, `nvs-ldap`, `nvs-lsp`, `nvs-render`, `nvs-repo`, `nvs-runtime`, `nvs-server`, `nvs-stdlib`, `nvs-syntax`, `nvs-test`, `nvs-types`. `Cargo.toml:36`, `crates/nvs-repo/Cargo.toml:10`, `crates/nvs-cli/Cargo.toml:1`
- [x] The same standing decision: `bun nv lints --check` refuses a crate manifest without the inherited `publish`, its header says so, and the test the `publish-false-test` check names. Run once by name. `tools/nv/cmd/lints.ts:1`, `tools/nv/test/lints.test.ts:28`
- [x] No rule governs this: the two descriptions the standing decision *Crate descriptions say what the crate does* gives. `crates/nvs-cli/Cargo.toml:3`, `crates/nvs-stdlib/Cargo.toml:3`
- [x] No rule governs this; the standing decision *`cache.rs` allows no dead code wholesale* is its spec. `#![allow(dead_code)]` and its comment go. Each warning `cargo clippy --all-targets -- -D warnings` names is gated to tests or deleted. `crates/nvs-cli/src/cache.rs:194`, `crates/nvs-cli/src/cache.rs:199`

Local:
- [x] Delete the ignored `a.out` at the repository root. `.gitignore:112`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the whole
goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and 19 gap records stand, each owned by a sibling goal. The acceptance sweep's two reds were already fixed: `bun nv proofs --gate --only
'Core\Env::secret'` exits 0, and nightly 41033290883 ran on `1e28a23c7`, which comes before the
`ExecutableFileBusy` fix `b9ce9d0e6`.

Verify ran four times. Run 1 was red on `nvs-stdlib test ldap`: seven Samba operations passed
their 30 s deadline together, the binary passed alone, no other worktree or lane used Samba, and
Samba logged no crashed worker. Run 2 was green. `bun nv ci --check` then failed on `cargo deny`'s
RUSTSEC-2026-0422, -0423 and -0424 against wasmtime 49.0.4, so `cf788ede6` moves the wasmtime floors
to 49.0.5 and regenerates `THIRD-PARTY-LICENSES.txt`. Run 3 was red on
`serve::tests::a_served_while_true_does_not_stall_a_second_connection_on_its_core`: it read the end
of a connection before a response head. It passed alone, and wasmtime does not reach it. Run 4 was
green. A playbook bullet records both load-only reds. `bun nv verify --doc`, `bun nv owners
--closes release-hygiene` and `bun nv ci --check` are green.

### Next

Nothing in this goal. The goal is reached.

### Backlog

- Sibling goals `registry-service`, `runtime-leftovers`, `spreadsheet-second-wave` and `extension-leftovers` close the
  records stages 2 and 3 wrote. Their `owners --closes` gates hold them (`tools/nv/cmd/owners.ts:469`).
- The two load-only reds above have no known cause yet. A goal that touches the Samba fixture
  (`crates/nvs-stdlib/tests/ldap.rs:163`) or the serve slice cases (`crates/nvs-cli/src/serve.rs:5993`)
  finds it. `dependency-sweep` takes the wasmtime 50 hold at `Cargo.toml:353`.
