---
looper: 1
title: "the website, the docs, the licenses and CI are ready for the first public release"
summary: "every should-fix finding of the 2026-10-30 pre-release review is fixed, and the checks below pass."
context: []
stages:
  - id: 1
    title: "the website's roadmap, install, footer, security and licenses pages are right, and every page has a social image"
    status: open
    context: [website/astro.config.ts, website/src/components/Footer.astro, website/src/components/Roadmap.astro, tools/nv/lib/site-roadmap.ts, website/config/site.mjs]
  - id: 2
    title: "the README, CONTRIBUTING, SECURITY and setup docs describe the released project, and the repo has a code of conduct and a PR template"
    status: open
    context: [README.md, CONTRIBUTING.md, SECURITY.md, docs/setup.md]
  - id: 3
    title: "the agent docs and rules cite only what exists, and every nvs command has a row in the CLI reference"
    status: open
    context: [docs/reference/tools/10-cli.md, crates/nvs-cli/tests/one_binary.rs, crates/nvs-cli/src/cache_cmd.rs, docs/rules/programs/slice-ranking.md]
  - id: 4
    title: "the .vsix and the binary carry every third-party notice they owe"
    status: open
    context: [tools/nv/cmd/gen-attribution.ts, editors/vscode/.vscodeignore, tools/nv/test/attribution-extensions.test.ts, docs/rules/packaging/the-third-party-notice-is-generated-never-written-by-hand.md]
  - id: 5
    title: "CI checks every lockfile and pins every download, and the release attests its checksums"
    status: open
    context: [.github/workflows/ci.yml, deny.toml, .github/workflows/release.yml, tools/nv/cmd/ci.ts, docs/rules/testing/ci-lanes.md]
  - id: 6
    title: "the editor extension reads as a published package, and the tools share one error helper"
    status: open
    context: [editors/vscode/package.json, editors/vscode/README.md, tools/nv/cmd/deps.ts, docs/agent/dependency-update.md]
  - id: 7
    title: "the lexical path normalizer, span_text and strip_sigil each exist once"
    status: open
    context: [crates/nvs-diagnostics/src/source.rs, crates/nvs-config/src/resolve.rs, crates/nvs-types/src/paths.rs, crates/nvs-diagnostics/src/embedded.rs]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: roadmap-tests
    stage: 1
    name: an empty roadmap renders one sentence in the README, and a full one renders its list
    run: [bun, test, tools/nv/test/site-roadmap.test.ts]
  - id: site-check
    stage: 1
    name: the website's structure, stamps, roadmap and prose checks pass on the changed pages
    run: [bun, nv, site, --check, structure, stale, roadmap, prose]
    timeout: 900
  - id: layout-check
    stage: 2
    name: CONTRIBUTING.md's layout block matches the tree
    run: [bun, nv, layout, --check]
  - id: readme-prose
    stage: 2
    name: the README follows the prose bounds of text an end user reads
    run: [bun, nv, prose, README.md]
  - id: code-of-conduct
    stage: 2
    name: the repository carries the Contributor Covenant
    file: CODE_OF_CONDUCT.md
    output_contains: ["Contributor Covenant", "version 2.1"]
  - id: pr-template
    stage: 2
    name: the repository carries a pull request template
    file: .github/pull_request_template.md
    min_bytes: 200
  - id: cli-reference-rows
    stage: 3
    name: every subcommand nvs --help lists has a row in the CLI reference's command table
    run: [cargo, test, --test, one_binary]
    tests:
      - every_command_nvs_help_lists_has_a_row_in_the_cli_reference
  - id: rules-check
    stage: 3
    name: the rulebook validates and its rendered chapters are current
    run: [bun, nv, rules, --check]
  - id: links
    stage: 3
    name: every link in the repository resolves
    run: [bun, nv, links]
    timeout: 600
  - id: attribution
    stage: 4
    name: the binary's notice and the extension's notice are what the generator writes
    run: [bun, nv, gen-attribution, --check]
    timeout: 900
  - id: vsix-notices
    stage: 4
    name: the extension's notice names every npm package bundled into out/extension.js, and the .vsix packs it
    run: [bun, test, tools/nv/test/attribution-vscode.test.ts]
  - id: supply-chain-coverage
    stage: 5
    name: CI's cargo-deny job and the pre-push checks cover every Cargo.lock in the tree, and deny.toml names every release target
    run: [bun, test, tools/nv/test/supply-chain.test.ts]
  - id: deny-image
    stage: 5
    name: cargo deny passes over the image extension's lockfile
    run: [cargo, deny, --manifest-path, extensions/image/Cargo.toml, check]
    timeout: 900
  - id: deny-intl
    stage: 5
    name: cargo deny passes over the intl extension's lockfile
    run: [cargo, deny, --manifest-path, extensions/intl/Cargo.toml, check]
    timeout: 900
  - id: deny-pdf
    stage: 5
    name: cargo deny passes over the pdf extension's lockfile
    run: [cargo, deny, --manifest-path, extensions/pdf/Cargo.toml, check]
    timeout: 900
  - id: deny-spreadsheet
    stage: 5
    name: cargo deny passes over the spreadsheet extension's lockfile
    run: [cargo, deny, --manifest-path, extensions/spreadsheet/Cargo.toml, check]
    timeout: 900
  - id: editor-lint
    stage: 6
    name: the extension's eslint config loads and passes
    run: [npm, run, lint]
    cwd: editors/vscode
    timeout: 600
  - id: editor-prose
    stage: 6
    name: the extension's README and CHANGELOG follow the prose bounds
    run: [bun, nv, prose, editors/vscode/README.md, editors/vscode/CHANGELOG.md]
  - id: deps-tests
    stage: 6
    name: bun nv deps reads holds from their new home and engines.bun as a range
    run: [bun, test, tools/nv/test/deps.test.ts]
  - id: lexical-normalize-tests
    stage: 7
    name: the one lexical normalizer keeps an unpoppable parent in a relative path and drops one at a root
    run: [cargo, test, --lib, lexical_normalize]
    timeout: 1800
    tests:
      - lexical_normalize_drops_a_current_dir_and_pops_a_parent
      - lexical_normalize_keeps_a_parent_it_cannot_pop_from_a_relative_path
      - lexical_normalize_drops_a_parent_at_a_root
  - id: ci-check
    stage: 8
    name: the checks the goal end's push runs are green
    run: [bun, nv, ci, --check]
    timeout: 3600
    reach: true
  - id: owners-closes
    stage: 8
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, release-polish]
    reach: true
    floor: false
---

# the website, the docs, the licenses and CI are ready for the first public release

## Why

The 2026-10-30 pre-release review read the project the way a first visitor reads it: the website, the
README, the repository's community files, the licenses of what ships, and CI. It found no blocker,
and many things a visitor trips over. The roadmap renders an empty list. The install page names the
wrong line of the help output. No page says how to report a security issue. The `.vsix` bundles npm
code with no notices. CI checks one lockfile of five. This goal fixes each finding.

It runs after the feature goals, so the docs describe the finished state: the package registry is
built (`registry-service`), and the runtime and spreadsheet work has landed. `release-run` follows it
and cuts the release.

## Standing decisions

**Already settled, and not reopened.** The sponsoring page keeps its "opens soon" text. Private
vulnerability reporting, secret scanning and push protection are on. `main` is the default branch,
and `landing` is gone. The root `LICENSE` (MIT) covers the whole repository, which `release-hygiene`
states. `release-hygiene` also owns the stale rule prose, the owners gap detection, the machine paths,
`publish = false`, the Cargo descriptions and `cache.rs`'s `dead_code`. This goal repeats none of them.
An item an earlier goal already fixed on disk is ticked with a note in the handoff, and never redone.

**Text an end user reads.** Every website page, the README, the extension's README and CHANGELOG, the
security and licenses pages and the new reference sections follow AGENTS.md § *Text an end user
reads*. CONTRIBUTING.md, SECURITY.md, the PR template and `docs/` are read by contributors, and are
written plainly but are outside that rule.

**The roadmap with no theme.** `Roadmap.astro` and `renderRoadmap` print one sentence when `themes` is
empty: "Nothing is planned after the current release yet." (or the wording the page's prose check
accepts). The page's own text says "is planned" only where a list follows. The roadmap still follows
the chain (the user's call), so the sentence appears only when the chain holds no public goal. Safe
fallback: if a theme exists when the stage runs, the sentence still lands, and the test pins both
cases.

**Security and contributing on the website.** A short top-level page, `/security/`, says how to
report a vulnerability: GitHub's private reporting form, the link SECURITY.md:21 gives, and what
happens next in two or three sentences. It links SECURITY.md for the full policy. The footer's
*Project* column gains *Security* (the page) and *Contributing* (CONTRIBUTING.md on GitHub, through
`githubFile`). No email address is added anywhere.

**Licenses and font credits.** A top-level page, `/licenses/`, replaces the footer's *License* link in
the *Legal* column. It says Novis is MIT and links `LICENSE`, says every copy of `nvs` carries its
third-party notices (`nvs info --licenses`, `THIRD-PARTY-LICENSES.txt`), and credits the two
self-hosted fonts, Plus Jakarta Sans and JetBrains Mono, each with its copyright and the full
OFL-1.1 text. The page reads that text at build time from each `@fontsource-variable/*` package's
`LICENSE`, so it never drifts. The website's fonts are not in the binary, so `THIRD-PARTY-LICENSES.txt`
does not list them. Like the legal notice, the page has `pagefind: false`.

**The social image.** One 1200x630 PNG, `website/public/social.png`: `media/novis-logo.png` centred
on the dark page ground `#111111`, with the site title beside it. A scratch script under `.agent-tmp/`
writes it once with `sharp` from `website/node_modules`, and is deleted. The PNG is committed, about
50 KB, served once per share and never on a page load. `astro.config.ts`'s `head` gets `og:image`
(absolute, from `SITE_URL`), `og:image:width`, `og:image:height`, `og:image:alt` and `twitter:image`.
Starlight's own `twitter:card` is kept if the installed version writes one; look it up in the
installed version's docs (AGENTS.md rule 13).

**The sitemap.** The legal notice and the privacy policy are `noindex`, so the sitemap leaves them
out. Use the installed Starlight's sitemap option if it has one. Safe fallback: add
`@astrojs/sitemap` with a `filter`, at a caret range, as Starlight's docs describe.

**The install page uses `--version`.** Both "Check that it runs" steps run `nvs --version`, which
prints one line, `nvs <version> (commit …)`. `nvs --help` prints `Novis-Lang` on line 1 and the
version on line 2 (`crates/nvs-cli/src/main.rs:218`), so a step that says "the first line" is wrong.
The download links keep reading `src/data/release.ts`, which reads `Cargo.toml`. The site deploys
with each release, so the links match the release that `release-run` cuts. No change there.

**The GitHub description.** The description is the home page's: "Novis is a programming language for
web applications and the command line. Build business apps, websites, APIs and web services. The tools
you need are built in." The homepage is `https://novis-lang.org`. If the loop's permissions refuse
`gh repo edit`, the session ticks the item with the exact command in the handoff, for the user. That
never blocks the goal.

**The code of conduct.** `CODE_OF_CONDUCT.md` is the Contributor Covenant 2.1, unchanged. Its
enforcement contact is the address on the website's legal notice
(`https://novis-lang.org/impressum/`), so the repository never carries the address in plain text,
which `website/config/site.mjs:18` already avoids on the pages. The PR template says pull requests
are not open yet (README.md § *Contributing*), and for when they are, asks what changed, which tests
ran and the trade-offs (AGENTS.md rule 11).

**`docs/goal-format.md` lives in looper.** No copy is in this tree, and the kit under `tools/looper/`
is code only. Each citation names it as "looper's `docs/goal-format.md` (in looper's repository)" and
points at the kit's code that carries the same fact in this tree: `tools/looper/src/goal/playbook.ts`
for the `until` kinds, `tools/looper/src/goal/model.ts` for a check's fields. Nothing is copied.

**"The plan" is retired.** Each citation of it as a document is reworded to the home that took its
place: the chain and the goals (`docs/goals/`), a rule, or `Cargo.toml` and `rust-toolchain.toml`
for versions. "The plan" in its plain sense, such as a schema migration plan or an image pipeline,
stays.

**Every command has a row.** The command table in `docs/reference/tools/10-cli.md` gets one row for
each subcommand `nvs --help` lists. A command whose section is in another chapter links to it. `nvs
cache` gets its own section beside *The compile cache*. The others without a section (`queue`,
`schema`, `tmp`, `doc`, `stubs`) keep the "not in this chapter yet" sentence. The test reads `nvs
--help`, so a new command fails it until it has a row.

**The extension's notice.** `bun nv gen-attribution` also writes `editors/vscode/THIRD-PARTY-NOTICES.txt`:
every npm package reachable from the extension's `dependencies` in `editors/vscode/package-lock.json`,
with its license text from `node_modules`, in the format of the binary's notice. That is
`vscode-languageclient`, `vscode-jsonrpc`, `vscode-languageserver-protocol`,
`vscode-languageserver-types`, `vscode-languageserver-textdocument`, `minimatch`, `brace-expansion`,
`balanced-match` and `semver`, or whatever the lockfile holds then. `src/` imports only `vscode`,
`node:` modules and `vscode-languageclient`, so that closure is what esbuild bundles. The test also
fails on a bare import in `editors/vscode/src/` outside `dependencies`, which keeps it so. `--check`
covers both files. The `.vsix` packs it, since
`.vscodeignore` does not exclude it. It fails closed on a missing license text, as the binary's notice
does. Safe fallback: if `node_modules` is missing in CI's attribution job, the job runs `npm ci` in
`editors/vscode` first.

**The WASI WIT files.** `wit/nvs-ext/deps/*.wit` are copies of the WASI 0.2 interfaces, embedded in
the binary through `crates/nvs-ext/src/pack.rs:51`. The session confirms the upstream and its license
from the copy `wasmtime-wasi` ships in `~/.cargo/registry` (expected: Apache-2.0 WITH LLVM-exception).
`wit/nvs-ext/deps/README.md` names the upstream, the version (`0.2.12`) and the license, and the
binary's notice gets one entry for them, through a `WIT_DIRS` list beside `FONT_DIRS`.

**The ext template's license.** `crates/nvs-cli/templates/ext/rust/Cargo.toml` is the user's project,
and its license is the user's to choose. It gets a commented `# license = "MIT"` line with one plain
comment line above it. It gets no license key. `packages/rest/package.toml` gets nothing: a package's
license is its `LICENSE` file (`docs/reference/tools/35-packages.md:403`), and `[package]` has no
`license` key.

**cargo-deny over the extensions.** The `supply-chain` job runs the action once per manifest: the root
and the four `extensions/*/Cargo.toml`. `PRE_PUSH` gets one `advisories` entry per extension
manifest. `deny.toml` is the config for all five; if cargo-deny does not find it from an extension's
manifest, each run names it with `--config`. `deny.toml`'s `targets` gains
`aarch64-unknown-linux-gnu`, `aarch64-pc-windows-msvc`, `x86_64-unknown-linux-musl` (build.yml:78-101
builds these) and `wasm32-wasip2` (what the extensions compile to). An advisory still failing after
`dependency-sweep` is fixed by an update, or ignored with a reason and a revisit date in the shape
`deny.toml:22` uses. MPL-2.0 stays allowed with a comment naming the extension crate that needs it.
The cranelift exception goes, since `Apache-2.0 WITH LLVM-exception` is in `allow`.

**The pins.** The wasi-sdk tarball is checked with `sha256sum -c` against one digest per matrix leg,
in a `wasi-sdk-sha256` matrix field beside `wasi-sdk`, in the shape ci.yml:231-246 uses for its zip.
`cargo-fuzz` is installed at an exact `--version` with `--locked`, the newest release when the stage
runs. The push trigger names `main` alone. `SHA256SUMS` gets an `actions/attest-build-provenance`
step in `publish`, at the SHA build.yml:222 pins, and `publish` gets `id-token: write` and
`attestations: write`.

**The extension's manifest.** `keywords` (`novis`, `nvs`, `language server`, `php`) and `bugs` (the
issues URL). The `holds` key leaves every `package.json`: each moves to a `holds.json` beside it,
`bun nv deps` reads only that file, and `docs/agent/dependency-update.md:43` says so. One rule for all
three `package.json` files, and the `.vsix` no longer ships a key no tool reads. `holds.json` goes in
`.vscodeignore`. `editors/vscode/CHANGELOG.md` is one short paragraph that points at the website's
*What changed* page, so no release has to edit it.

**Bun's version.** The exact pin moves to `"packageManager": "bun@1.4.3"`, which `setup-bun`'s
`bun-version-file` reads first, and `engines.bun` becomes `>=1.4.3`. CI keeps one exact Bun, and a
contributor with a newer Bun is not refused. Look up `setup-bun`'s behaviour in its docs first. Safe
fallback: if `setup-bun` cannot read `packageManager`, `engines.bun` stays exact, and
`docs/agent/dependency-update.md:43` says why.

**One error helper.** `messageOf(e: unknown): string` moves to `tools/nv/lib/proc.ts`, beside the
other process helpers, and every inline `e instanceof Error ? e.message : String(e)` under `tools/nv/`
calls it. `bench-docker.ts`'s export goes, and its two importers import from `lib/proc.ts`.

**One lexical normalizer.** It goes in `nvs-diagnostics`, the lowest crate that `nvs-config` and
`nvs-types` both depend on, as `pub fn lexical_normalize(path: &Path) -> PathBuf` in a new
`crates/nvs-diagnostics/src/paths.rs`. It drops `.`, pops a `..`, keeps a `..` it cannot pop from a
relative path, and drops one at a root. `nvs-config` keeps its empty-to-`.` step as a wrapper. The
`nvs-types` copy dropped a leading `..` of a relative path. It now keeps it, which is the safer
direction: a dropped `..` reads a different file than the one written. Trade-offs: no request-path
cost, no memory, one function instead of three.

**One `span_text` and one `strip_sigil`.** `nvs_diagnostics::span_text(src, span) -> &str` returns
`""` for a foreign span, and sits beside `SourceFile::span_text` (`crates/nvs-diagnostics/src/source.rs:248`).
`nvs_syntax::strip_sigil` is public. Each crate that had a copy re-exports the shared one where the
copy was (`pub(crate) use`), so none of the 365 callers changes.

**Lanes.** Stages 1 and 2 share `README.md` (the roadmap sentence, then the quick start), so stage 1
runs first. Stages 4 and 6 share `editors/vscode/` (`.vscodeignore`, `package.json`), so stage 4 runs
first. Stages 3, 5 and 7 touch disjoint files and may run as side goals. The goal depends on
`registry-service` for `website/src/content/docs/in-depth/new.mdx:27` and CONTRIBUTING.md:138, gated
by `docs/goals/registry-service.md` being gone, and on `dependency-sweep` for the extension
lockfiles, gated by `docs/goals/dependency-sweep.md` being gone.

## Stages

### 1. the website's roadmap, install, footer, security and licenses pages are right, and every page has a social image

**Does:** a first visitor to the website finds no empty list, no wrong install step, and a link for
security, contributing and licenses on every page.

- [ ] An empty roadmap renders one sentence on the page and in the README, and the roadmap page says "is planned" only above a list. `website/src/components/Roadmap.astro:23`, `website/src/content/docs/in-depth/roadmap.mdx:9`, `website/src/content/docs/in-depth/roadmap.mdx:15`, `tools/nv/lib/site-roadmap.ts:69`, `README.md:121`
- [ ] The test the `roadmap-tests` check runs gains "an empty roadmap renders one sentence". Then `bun nv site --roadmap`. `tools/nv/test/site-roadmap.test.ts:30`
- [ ] The *What is new* page describes the package registry as present, with a link to its page. `website/src/content/docs/in-depth/new.mdx:27`
- [ ] The install page's two "Check that it runs" steps run `nvs --version`. `website/src/content/docs/install/index.mdx:53`, `website/src/content/docs/install/index.mdx:108`
- [ ] The `/security/` page. `website/src/content/docs/security.mdx:1`
- [ ] The `/licenses/` page with the two font credits and the OFL text read at build time. `website/src/content/docs/licenses.mdx:1`
- [ ] The footer: *Security* and *Contributing* in *Project*, *Licenses* in *Legal*. `website/src/components/Footer.astro:22`, `website/src/components/Footer.astro:35`
- [ ] The social image and its head tags. `website/public/social.png:1`, `website/astro.config.ts:147`
- [ ] The legal notice and privacy policy are out of the sitemap. `website/astro.config.ts:57`, `website/src/content/docs/impressum.mdx:11`, `website/src/content/docs/datenschutz.mdx:11`
- [ ] `bun nv site --stamp` on every changed page, and the `site-check` check passes. `tools/nv/cmd/site.ts:1989`

### 2. the README, CONTRIBUTING, SECURITY and setup docs describe the released project, and the repo has a code of conduct and a PR template

**Does:** the files GitHub shows a visitor say what is true at the release, and the community files
GitHub looks for exist.

- [ ] SECURITY.md says eight audited `unsafe` crates (the roster in `tools/nv/cmd/lints.ts:73`), and the platforms CI runs on. `SECURITY.md:66`, `SECURITY.md:109`, `SECURITY.md:111`
- [ ] CONTRIBUTING.md's status names no version and no "live one", and points at the roadmap. `CONTRIBUTING.md:10`
- [ ] CONTRIBUTING.md's start block gives the `bun tools/looper/main.ts orient --full` fallback and lists the prerequisites: Rust (from `rust-toolchain.toml`), Bun, Node for the extension, and Docker for the database and bench tests. `CONTRIBUTING.md:89`
- [ ] CONTRIBUTING.md no longer names the package registry as unbuilt, and a new decision "goes into a rule", with no "plan". `CONTRIBUTING.md:138`, `CONTRIBUTING.md:245`
- [ ] README.md: a quick start of four lines under the status: the install link, the hello world from `website/snippets/install/01-hello.nvs`, and `nvs run hello.nvs`. `README.md:8`
- [ ] README.md: the maintainer commands (`bun nv gen-attribution`) leave the License section for CONTRIBUTING.md. `README.md:161`, `README.md:170`
- [ ] docs/setup.md: a short path for an outsider first (clone, Rust, Bun, `cargo build`, `bun nv verify --fast`), then the loop operator's material. `docs/setup.md:1`, `docs/setup.md:54`
- [ ] docs/release.md: the broken sentence "What it means for `bump`." is a whole sentence. `docs/release.md:176`
- [ ] `CODE_OF_CONDUCT.md`, the Contributor Covenant 2.1. `CODE_OF_CONDUCT.md:1`
- [ ] The PR template. `.github/pull_request_template.md:1`
- [ ] The GitHub description and homepage: `gh repo edit novis-lang/novis --description "<the home page's description>" --homepage https://novis-lang.org`. If refused, the handoff names the command. `website/src/content/docs/index.mdx:4`

### 3. the agent docs and rules cite only what exists, and every nvs command has a row in the CLI reference

**Does:** no doc cites the retired plan or a file that is not in the tree, and the CLI reference's
table lists every command the binary has.

**Split:** none. The edits are one line each.

- [ ] "The plan" as a document is reworded in each agent doc. `docs/agent/user-report.md:134`, `docs/agent/dependency-update.md:163`, `docs/agent/doc-cleanup.md:68`
- [ ] "The plan" in two rules and one playbook bullet. Then `bun nv rules --render`. `rule:packaging/the-version-contract-starts-at-0-1-0`, `rule:programs/slice-ranking`, `docs/rules/packaging/the-version-contract-starts-at-0-1-0.md:10`, `docs/rules/programs/slice-ranking.md:2`, `docs/agent/playbook/writing-novis/a-slice-the-plan-files-under-nvs-ir-can-be-unbuildable-there.md:1`
- [ ] Each `docs/goal-format.md` citation names looper's repository and the kit file in this tree. `docs/agent/commands.md:44`, `docs/agent/conventions.md:603`, `docs/agent/coordinator.md:53`, `docs/agent/loop-authoring.md:4`, `docs/agent/playbook.md:6`
- [ ] `# nvs cache`: what `nvs cache gc` and `nvs cache clear` do, and `--json`. `docs/reference/tools/10-cli.md:1151`, `crates/nvs-cli/src/cache_cmd.rs:43`
- [ ] The command table has a row for every subcommand, and the "In other chapters" and "Not in this chapter yet" paragraphs shrink to what the rows do not say. `docs/reference/tools/10-cli.md:20`, `docs/reference/tools/10-cli.md:41`
- [ ] The test the `cli-reference-rows` check names. `crates/nvs-cli/tests/one_binary.rs:29`
- [ ] `bun nv render --website` writes the new section into the website's CLI reference, and the CLI tools page links it. `website/src/data/reference.json:1`, `website/src/content/docs/tools/cli/index.mdx:75`

### 4. the .vsix and the binary carry every third-party notice they owe

**Does:** every component that ships in the `.vsix` or the binary has its notice, and
`bun nv gen-attribution --check` holds both.

- [ ] `gen-attribution` writes `editors/vscode/THIRD-PARTY-NOTICES.txt` from the extension's runtime npm packages, in the way `viewerPackages` reads the viewer's. `rule:packaging/the-third-party-notice-is-generated-never-written-by-hand`, `tools/nv/cmd/gen-attribution.ts:299`, `tools/nv/cmd/gen-attribution.ts:926`, `editors/vscode/THIRD-PARTY-NOTICES.txt:1`
- [ ] `.vscodeignore` keeps the notice in the `.vsix`, and CI's attribution job installs the extension's packages if it needs them. `rule:testing/attribution-is-diffed-in-ci`, `editors/vscode/.vscodeignore:1`, `.github/workflows/ci.yml:986`
- [ ] The test the `vsix-notices` check runs. `tools/nv/test/attribution-vscode.test.ts:1`
- [ ] `wit/nvs-ext/deps/README.md` with the upstream, version and license, and the WIT entry in the binary's notice. `wit/nvs-ext/deps/README.md:1`, `tools/nv/cmd/gen-attribution.ts:279`, `crates/nvs-ext/src/pack.rs:51`
- [ ] The ext template's commented license line. `crates/nvs-cli/templates/ext/rust/Cargo.toml:5`
- [ ] `bun nv gen-attribution`, and the regenerated `THIRD-PARTY-LICENSES.txt` is committed. `THIRD-PARTY-LICENSES.txt:1`

### 5. CI checks every lockfile and pins every download, and the release attests its checksums

**Does:** nothing CI downloads runs unchecked, every lockfile in the tree is audited, and a user can
verify `SHA256SUMS` like the archives.

- [ ] `supply-chain` runs cargo-deny over the root and the four extension manifests. `rule:testing/ci-lanes`, `.github/workflows/ci.yml:986`, `.github/workflows/ci.yml:1000`
- [ ] `PRE_PUSH` gets one advisories entry per extension manifest. `tools/nv/cmd/ci.ts:103`, `tools/nv/cmd/ci.ts:110`
- [ ] `deny.toml`: the four new targets, the MPL-2.0 comment, and no cranelift exception. `deny.toml:7`, `deny.toml:58`, `deny.toml:65`
- [ ] Any advisory the four `deny-*` checks name is fixed or ignored with a reason and a date. `deny.toml:22`, `extensions/image/Cargo.lock:1`, `extensions/intl/Cargo.lock:1`, `extensions/pdf/Cargo.lock:1`, `extensions/spreadsheet/Cargo.lock:1`
- [ ] The wasi-sdk tarball is checked against a pinned SHA256 per leg. `.github/workflows/ci.yml:671`, `.github/workflows/ci.yml:709`
- [ ] `cargo-fuzz` at an exact version. `.github/workflows/ci.yml:1234`
- [ ] The push trigger names `main` alone. `.github/workflows/ci.yml:5`
- [ ] `SHA256SUMS` is attested in `publish`. `.github/workflows/release.yml:260`, `.github/workflows/release.yml:363`
- [ ] The test the `supply-chain-coverage` check runs: it globs every `Cargo.lock`, and fails naming one that `ci.yml` or `PRE_PUSH` does not cover, or a `build.yml` target `deny.toml` does not list. `tools/nv/test/supply-chain.test.ts:1`

### 6. the editor extension reads as a published package, and the tools share one error helper

**Does:** the Marketplace page of the extension reads like a user's page, and the tools lose small
duplicates and dead files.

- [ ] The extension's README says what the extension does for the user, with no rule id and no internals. `editors/vscode/README.md:8`
- [ ] `keywords` and `bugs`, and `editors/vscode/CHANGELOG.md`. `editors/vscode/package.json:14`, `editors/vscode/CHANGELOG.md:1`
- [ ] `holds` moves to a `holds.json` beside each `package.json`, `bun nv deps` reads it, and the extension's is in `.vscodeignore`. `editors/vscode/package.json:590`, `tools/nv/cmd/deps.ts:25`, `tools/nv/cmd/deps.ts:160`, `docs/agent/dependency-update.md:43`
- [ ] `eslint.config.mjs` uses `defineConfig` from `eslint/config`, without the deprecated `semi` rule. `editors/vscode/eslint.config.mjs:4`, `editors/vscode/eslint.config.mjs:18`
- [ ] Bun's exact pin moves to `packageManager`, and `engines.bun` is a range. `package.json:9`
- [ ] `messageOf` in `tools/nv/lib/proc.ts`, and every copy calls it. `tools/nv/lib/proc.ts:1`, `tools/nv/cmd/bench-docker.ts:11`, `tools/nv/cmd/bench.ts:825`, `tools/nv/accept/origin.ts:83`, `tools/nv/cmd/deps.ts:498`, `tools/nv/cmd/render.ts:26`, `tools/nv/cmd/scaling.ts:947`, `tools/nv/cmd/scaling.ts:1002`, `tools/nv/cmd/webp-lib.ts:199`, `tools/nv/proofs/run.ts:203`, `tools/nv/proofs/run.ts:649`, `tools/nv/proofs/run.ts:817`
- [ ] `tools/data/calibration.json` is deleted: its only reader was the retired `orient.py`. `tools/data/calibration.json:1`
- [ ] `leak-check.sh` and `tsan.sh` show the plain Linux command beside the `wsl.exe` one. `tools/leak-check.sh:11`, `tools/tsan.sh:8`

### 7. the lexical path normalizer, span_text and strip_sigil each exist once

**Does:** three copies of each helper become one, with every caller's behaviour pinned by a test.

- [ ] `nvs_diagnostics::lexical_normalize` and the tests the `lexical-normalize-tests` check names. `crates/nvs-diagnostics/src/paths.rs:1`, `crates/nvs-diagnostics/src/lib.rs:34`
- [ ] The three copies call it, and `nvs-config` keeps its `.` step. `crates/nvs-config/src/resolve.rs:1078`, `crates/nvs-diagnostics/src/embedded.rs:220`, `crates/nvs-types/src/paths.rs:430`
- [ ] `nvs_diagnostics::span_text`, re-exported where each copy was. `crates/nvs-diagnostics/src/source.rs:248`, `crates/nvs-ir/src/lib.rs:646`, `crates/nvs-types/src/lib.rs:703`, `crates/nvs-syntax/src/casing.rs:85`
- [ ] `nvs_syntax::strip_sigil`, re-exported where each copy was. `crates/nvs-syntax/src/casing.rs:208`, `crates/nvs-ir/src/lib.rs:652`, `crates/nvs-types/src/lib.rs:708`
- [ ] The phi comment says what the code does, without "for now". `crates/nvs-ir/src/lower/control.rs:149`

### 8. the goal is verified once, and every failure is fixed

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

Nothing is built yet. Every anchor was checked against the tree on 2026-10-30. Stage 1 waits for
`registry-service` only for its `new.mdx` item, and stage 5's `deny-*` checks wait for
`dependency-sweep`.

### Next

Stage 1, the website. The slices share `website/astro.config.ts`, `website/src/components/` and
`tools/nv/lib/site-roadmap.ts`. Gate: `docs/goals/registry-service.md` is gone, for the `new.mdx`
item alone.

- [ ] An empty roadmap renders one sentence. `website/src/components/Roadmap.astro:23`, `website/src/content/docs/in-depth/roadmap.mdx:9`, `tools/nv/lib/site-roadmap.ts:69`, `README.md:121`
- [ ] The roadmap test's new case, then `bun nv site --roadmap`. `tools/nv/test/site-roadmap.test.ts:30`
- [ ] The registry is present on *What is new*. `website/src/content/docs/in-depth/new.mdx:27`
- [ ] The install steps run `nvs --version`. `website/src/content/docs/install/index.mdx:53`, `website/src/content/docs/install/index.mdx:108`
- [ ] The `/security/` and `/licenses/` pages and the footer links. `website/src/content/docs/security.mdx:1`, `website/src/content/docs/licenses.mdx:1`, `website/src/components/Footer.astro:22`
- [ ] The social image and the sitemap filter. `website/public/social.png:1`, `website/astro.config.ts:147`, `website/astro.config.ts:57`
- [ ] Stamps, and the `site-check` check. `tools/nv/cmd/site.ts:1989`

No test is expected to go red.

### Backlog

- `crates/nvs-server/src/serve.rs` is 14,400 lines, and `nvs-stdlib` is the largest crate: a split waits for a measured cost (`docs/agent/coding.md`).
- The website builds twice per push, in `.github/workflows/pages.yml` and in `ci.yml`'s `proofs` job (`rule:testing/ci-lanes`).
