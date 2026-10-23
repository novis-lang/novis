---
looper: 1
title: "Novis is ready for its first release, and the README and the website say Pre-Alpha and show the roadmap that is left"
summary: "bun nv release names the first release 0.0.1 with written notes, every release attaches the VS Code extension at its own version, the README matches the website's home page and its prose, every page says Pre-Alpha and released, the public roadmap is a short list of the planned work kept in step with the goal chain by a check, the website deploys again after a release is published, and the checks below pass."
context: [docs/release.md, tools/nv/cmd/release.ts, README.md, website/src/content/docs/index.mdx]
stages:
  - id: 1
    title: "the first release is 0.0.1 with written notes and the extension attached, and a published release redeploys the website"
    status: done
    context: [tools/nv/test/release.test.ts, .github/workflows/release.yml, .github/workflows/release-promote.yml, editors/vscode/package.json]
  - id: 2
    title: "the roadmap is a short list kept in step with the chain, and every page says Pre-Alpha and released"
    status: done
    context: [website/src/content/docs/in-depth/roadmap.mdx, website/src/content/docs/in-depth/what-changed.mdx, website/src/content/docs/install/index.mdx, website/src/content/docs/guides/production/docker.mdx, tools/nv/cmd/site.ts]
  - id: 3
    title: "the README follows the website's home page, in its order and its words"
    status: done
    context: [website/src/components/home/Hero.astro, website/src/components/home/Facts.astro, website/src/content/docs/sponsoring.mdx, CONTRIBUTING.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: release-tests
    stage: 1
    name: the first release is 0.0.1 and its notes are the written file
    run: [bun, test, tools/nv/test/release.test.ts]
  - id: roadmap-check
    stage: 2
    name: the roadmap names every planned goal once, in chain order, and the README's copy matches
    run: [bun, nv, site, --check, roadmap]
---

# Novis is ready for its first release, and the README and the website say Pre-Alpha and show the roadmap that is left

## Why

The release pipeline exists and GitHub is set up for it (2026-10-23: the `release` environment, the
default branch, the permissions). Three things in the tree still stand between it and a first release.

- `bun nv release --preview patch` names the first release `0.0.2`, although `0.0.1` was never
  released. With no earlier tag, its notes are the whole commit log: 427 lines, 85 of them naming the
  old project name or decision records. That text breaks § *Text an end user reads* in AGENTS.md.
- Every status line says "no release yet": the README, CONTRIBUTING, SECURITY, the roadmap, What
  changed and the install page. After the release they are wrong.
- The public roadmap lists what is built, and plans that are not on the goal chain. The user's call
  (2026-10-23): the roadmap is a short summary of the planned goals. It shows only what is planned or
  in progress, never what is built.

The user also asked that the README follow the website's home page, with the same order of features.
Where the README has a heading the website also has, it uses the website's words. The website's prose
is the correct one.

The release itself is not part of this goal. A person starts the release workflow, and no agent does
(`docs/release.md`). § *After the goal* below is what happens then.

## Standing decisions

**The status word is "Pre-Alpha", always with that capitalization.** Every place that says the status
says "Pre-Alpha" and that Novis is released. It also says what Pre-Alpha means, in one plain sentence:
any release can change the language, and Novis is not ready for production use. The
"no release yet" sentences go.

**The first release is `0.0.1`.** `Cargo.toml` has said `0.0.1` since the start, and no tag exists.
So when no `v*` tag exists, `resolveNext` releases the current version as it is: `patch` and `minor`
give `0.0.1`. `major` still gives `0.1.0` and still needs `allow_contract`. Once a tag exists, nothing
changes. The `apply` path commits the changelog alone when the manifests need no bump.

**The first release's notes are a written file, not the commit log.** With no previous tag,
`notesFor` returns the contents of `docs/release-notes/first.md` under the version heading, and the
CHANGELOG gets the same section. The file is text an end user reads, at most 40 lines. It says what
Novis is in the home page's words, and what Pre-Alpha means. It says how to install Novis, from the
archives or the Docker image. It links to the roadmap and to What changed. Every later release renders the commit log as
today.

**A published release redeploys the website.** The `publish` job pushes with `GITHUB_TOKEN`, and that
push starts no workflow. So the site would show the old version until the next push to `main`.
`release-promote.yml` dispatches `pages.yml` on `main` once the tags have moved. A dispatch is one of
the two events a `GITHUB_TOKEN` may start. The job gains `actions: write` and nothing else.

**Every release attaches the VS Code extension as `nvs-<version>.vsix`.** The user's call
(2026-10-23): publish the `.vsix` with each release, and not in the Marketplace for now. So no account,
token or secret is added. A `vsix` job in `release.yml` runs after `plan`. It applies the version with
`--manifests-only`, then runs `npm ci` and `npm run package` in `editors/vscode`, and attests the file.
`publish` downloads it beside the archives. It is listed in `SHA256SUMS` and attached to the draft,
and `publish` refuses a release without it. The `test` step already ran the extension's suites on the
same commit, so this job runs none. Building the file is not repeated in CI.

**The release bumps the extension's version too.** Today `bumpManifest` rewrites `Cargo.toml` alone,
and `editors/vscode/package.json` stays at `0.0.1`. The contributions suite asserts that the two agree
(`editors/vscode/test/contributions/contributions.test.ts:217`), so the second release would turn CI
red. `--apply` and `--manifests-only` also rewrite the `version` of `package.json` and the two
top-level ones in `package-lock.json`, and die if any of the three is missing.

**The website reads the version from `Cargo.toml` at build time.** One module,
`website/src/data/release.ts`, exports the workspace version and the archive names built from it. The
status label, the install page's download links and the Docker guide's tags read it. No page writes a
version number by hand. The archive name is `nvs-<version>-<target>`, as `packageBinary` writes it
(`tools/nv/cmd/release.ts:537`).

**The roadmap is data with one home, `website/src/data/roadmap.json`.** It is a list of themes. Each
theme has a title, one or two plain sentences, and the goal slugs it summarizes. The roadmap page
renders it with a component, and `bun nv site --roadmap` writes the same list into the README between
two marker comments. The first theme is labelled "Working on it now". No theme says what is built,
and no theme gives a date.

**`bun nv site --check roadmap` keeps it in step with the chain.** It fails in five cases:

- a goal on the chain is in no theme;
- a theme names a goal that is not on the chain;
- a theme has no goals left;
- the themes are not in the chain order of their first goal;
- the README's copy differs.

`internal` in the same file lists the goals that are not public work: today only `first-release`.
So a new goal joins a theme when it joins the chain. A theme leaves the page when its last goal is
retired. `bun nv affected` maps `docs/goals/README.md` and `roadmap.json` to that part.

**The themes, in chain order** (the plans "A faster compiler" and "A web framework" leave the page,
because no goal plans them):

| Theme | Goals |
|---|---|
| Formatter and command line | `formatter-complete`, `cli-housekeeping`, `telemetry-service` |
| Editor | `editor-refactors`, `editor-fixes`, `request-aware-editor` |
| Web APIs | `api-document`, `query-binding` |
| Testing | `test-surface`, `mutation-testing`, `bench-counters` |
| Debugging and profiling | `trace-timeline`, `debugger-core`, `debugger-adapter`, `debug-service`, `debug-viewer`, `tools-website` |
| Images | `image-second-wave` |
| Packages and the registry | `namespace-grants`, `api-compat`, `package-client`, `registry-log`, `registry-service`, `registry-website`, `package-audit`, `first-packages` |
| Passkeys and Web Push | `core-passkey`, `core-web-push` |

Each theme's sentences say what a reader will be able to do, in the prose rules' words. The goal titles
are written for agents and are not copied.

**The README follows the home page.** Its feature list has the home page's sections in the home
page's order:

1. the hero and the facts;
2. the compiler finds the mistake first;
3. small programs, clear rules;
4. one request cannot break another;
5. one library comes with it;
6. one program runs, tests and serves your code;
7. coding agents;
8. measured and counted.
 Each item uses the website's sentence for the same point and links to
the website page that explains it (`https://novis-lang.org/...`), never to `docs/rules/`. Where the
README has a heading the website also has (*Sponsoring*, the roadmap, the status), the README takes the
website's text, shortened. *Contributing* and *Licence* have no website page and stay, rewritten to
the prose rules. The whole README is held to § *Text an end user reads*: it is the first page most
readers see.

**CONTRIBUTING and SECURITY change only their status lines.** Their developer half stays as it is.
SECURITY says which release gets security fixes: the latest Pre-Alpha release and `main`.

**Lanes.** One lane. Stage 1 owns the release tool and the workflows. Stage 2 and stage 3 both write
`README.md`, so they run in order.

**Trade-offs.** Security: none changes. The promote job gains `actions: write`, the smallest grant that
can start a workflow. Latency: nothing on the request path. Memory: none. Simplicity: one more data
file and one more check, which replace a page that went stale without anybody noticing. A new goal
costs one line in `roadmap.json`.

## Stages

### 1. the first release is 0.0.1 with written notes and the extension attached, and a published release redeploys the website

**Does:** `bun nv release --preview patch` prints `0.0.1 --patch--> 0.0.1` and the written notes. A
release attaches the extension at its own version, and publishing it redeploys the website.

- [x] With no `v*` tag, `resolveNext` releases the current version for `patch` and `minor`, and `major` still needs `allow_contract`. `tools/nv/cmd/release.ts:624`
- [x] `apply` commits the changelog alone when no manifest needs a bump. `tools/nv/cmd/release.ts:667`
- [x] With no previous tag, `notesFor` returns `docs/release-notes/first.md` under the version heading. `tools/nv/cmd/release.ts:603`
- [x] Write `docs/release-notes/first.md` to the prose rules, from the home page's words, at most 40 lines. `docs/release-notes/first.md:1`
- [x] Tests: with no tag, `patch` and `minor` give `0.0.1` and `major` alone dies. No previous tag gives the written notes, and a tag gives the commit log. Run once by name: `bun test tools/nv/test/release.test.ts`. `tools/nv/test/release.test.ts:1`
- [x] `release-promote.yml` dispatches `pages.yml` on `main` after the tags move, with `actions: write` on that job alone, and no `${{ }}` inside a `run:` block. `.github/workflows/release-promote.yml:61`
- [x] `--apply` and `--manifests-only` rewrite the version in `editors/vscode/package.json` and `editors/vscode/package-lock.json`, with a test beside the others. `tools/nv/cmd/release.ts:216`, `tools/nv/test/release.test.ts:1`
- [x] A `vsix` job builds and attests `nvs-<version>.vsix`. `publish` requires it, lists it in `SHA256SUMS` and attaches it to the draft. `.github/workflows/release.yml:292`, `.github/workflows/release.yml:437`, `.github/workflows/release.yml:452`
- [x] `docs/release.md`: the version table says the first release is `0.0.1`, step 6 says the website redeploys, and the setup sections say GitHub is set up. *What an archive carries* names the `.vsix`. `docs/release.md:1`
- [x] The extension's README says each release attaches the `.vsix`, and that it is not in the Marketplace. `editors/vscode/README.md:67`

### 2. the roadmap is a short list kept in step with the chain, and every page says Pre-Alpha and released

**Does:** the roadmap page lists only planned work from `roadmap.json`. The check fails on a chain it
does not match, and no page says "no release yet".

- [x] `website/src/data/roadmap.json` with the themes above, their sentences and `internal`. `website/src/data/roadmap.json:1`
- [x] `website/src/data/release.ts`: the version from `Cargo.toml`, and the archive name for each target. `website/src/data/release.ts:1`
- [x] A roadmap component renders the themes, the first one labelled "Working on it now". The roadmap page uses it and loses *What is built*. `website/src/content/docs/in-depth/roadmap.mdx:1`
- [x] `bun nv site --roadmap` writes the README's roadmap list between the markers, and `bun nv site --check roadmap` runs the five checks above. A test covers each failure. `tools/nv/cmd/site.ts:1`
- [x] `bun nv affected` maps `docs/goals/README.md` and `website/src/data/roadmap.json` to the roadmap check. `tools/nv/cmd/affected.ts:1`. No code: `affected` maps a `bun nv` check by the reads its last recorded run made, and `site --check roadmap` reads both. The sweep's first run of `roadmap-check` records them.
- [x] The install page leads with the release archives and the Docker image, with download links from `release.ts`. Building from source comes after. *Check a release archive* uses the real archive name. `website/src/content/docs/install/index.mdx:95`
- [x] What changed and the roadmap page lose "There is no release yet". What changed says the first release is `0.0.1`. `website/src/content/docs/in-depth/what-changed.mdx:13`, `website/src/content/docs/in-depth/roadmap.mdx:37`
- [x] The install page's description and opening say you download `nvs` or build it from source. `website/src/content/docs/install/index.mdx:4`, `website/src/content/docs/install/index.mdx:9`
- [x] The home page's install card says download, with build from source second. `website/src/components/home/Path.astro:25`
- [x] The hero shows a small "Pre-Alpha" label with the version from `release.ts`, linking to What changed. `website/src/components/home/Hero.astro:1`
- [x] The Docker guide pulls the published image first, and building your own image comes after. Its tags come from `release.ts`. `website/src/content/docs/guides/production/docker.mdx:20`, `website/src/content/docs/guides/production/docker.mdx:139`
- [x] The editor page installs the `.vsix` from the release, with its link from `release.ts`, by *Extensions: Install from VSIX* or `code --install-extension`. Building the file from source comes after. `website/src/content/docs/install/editor.mdx:35`
- [x] The In depth index describes the roadmap as the planned work. `website/src/content/docs/in-depth/index.mdx:22`
- [x] The compiler page no longer names a planned faster compiler, because the roadmap does not have one. `website/src/content/docs/in-depth/concepts/compiled.mdx:35`
- [x] `CONTRIBUTING.md` says Pre-Alpha and released at `0.0.1`, and its "not built yet" example names planned work. `CONTRIBUTING.md:3`, `CONTRIBUTING.md:9`, `CONTRIBUTING.md:138`
- [x] `SECURITY.md`'s table supports the latest release and `main`, and its status line says Pre-Alpha. `SECURITY.md:9`, `SECURITY.md:14`
- [x] `bun nv proofs --comments` and `bun nv site --check` pass on every page this stage touched. `prose`, `structure`, `in-depth` and `roadmap` pass, `bun nv prose` passes on the pages, and `bun nv site --build` is green. The stage touched no `.nvs` file.

These stay as they are, and were checked on 2026-10-23. `in-depth/new.mdx:27` (the registry is still
planned). `how-novis-is-tested.mdx:496` (test practice, not the roadmap). `Sponsors.astro`: its
progress list is empty. The generated `llms.txt` follows the page descriptions by itself.

**Split:** after the first four items land, two writing subagents run side by side. One takes the
website pages and components, and one takes `CONTRIBUTING.md` and `SECURITY.md`. Neither builds.

### 3. the README follows the website's home page, in its order and its words

**Does:** the README reads like a short version of the home page, in its order and with its sentences.
Every link goes to the website.

- [x] The title, the opening paragraph and the status take the home page's hero and facts, with the Pre-Alpha line. `README.md:1`, `website/src/components/home/Hero.astro:1`, `website/src/components/home/Facts.astro:1`
- [x] The feature list follows the home page's sections in their order, each with the website's sentence and a link to its page. `README.md:12`
- [x] The roadmap section is the generated list and a link to the roadmap page. `README.md:46`
- [x] *Sponsoring* is a short version of the sponsoring page, and links to it. `README.md:80`, `website/src/content/docs/sponsoring.mdx:1`
- [x] *Contributing* and *Licence* follow the prose rules, and keep what they say. `README.md:63`, `README.md:108`
- [x] `bun nv site --check roadmap` passes, and `bun nv proofs --comments README.md` passes.

### 4. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## After the goal

These steps are the user's, with an interactive session. The loop takes none of them.

1. Push `main`, and wait for a full CI run on it to be green. A red run is fixed first, by the side
   goal `ci-green`. The release workflow runs the whole of CI as its test step.
2. Run the release workflow with `dry_run` ticked, and fix what it finds.
3. Pause the loop. Run the release workflow again without `dry_run`, approve the `release`
   environment, and publish the draft.
4. Make the `novis` container package public, once (`docs/release.md` § 6). Then check `docker pull`,
   `docker run ghcr.io/novis-lang/novis:0.0.1 nvs --version`, the checksums of one archive, and the
   install page's links. Install `nvs-0.0.1.vsix` from the release in VS Code, and open a `.nvs` file.

## Handoff

### State

All four stages are done and committed. The gate ran `bun tools/looper/main.ts verify`, because the
installed `looper` binary has no `verify` or `check` subcommand. It found one failure. The
extension's contributions suite asserts that `displayName` is the README's H1, and stage 3 made the
H1 `Novis-Lang`. `d77fa9b34` sets the extension's `displayName` and the `nvs --help` banner to
`Novis-Lang`. A unit test in `crates/nvs-cli/src/main.rs` now holds the banner to the H1. One rerun
stopped on a Windows lock on `target/covws/.../nvs.exe`. No process held it afterwards, and the next
run was green. The final verify is green.

The roadmap check went red at every retirement, because the driver commits only the goals directory.
`24144805f` adds `bun nv site --roadmap --retired` as the first `after_retire` hook in `looper.toml`.
It drops the retired goals and any themes left with no goals, then commits. Here it drops
`first-release` from `internal`. No hand edit to `roadmap.json` is owed.

The goal-end gates pass: `bun nv verify --doc` (every link resolves) and `bun nv owners --closes
first-release` (no gap). The acceptance checks pass: `release-tests` (20 pass) and `roadmap-check`.
The decisions only this file held are now in `docs/agent/conventions.md` § *The README and the
status*. The release, version and roadmap decisions already live in the headers of
`tools/nv/cmd/release.ts`, `website/src/data/release.ts` and `tools/nv/lib/site-roadmap.ts`.

The website stale guard is green locally. `aecce6733` restamped the pages that CI run `39982774098`
named. The sweep's nightly reds ran at `7b6ef53ee`, and later commits fixed those cases: `ba8930b80`,
`342675093` and `dc3a2ed29`. `env-secret-proofs` exits 0.

The first DONE was refused because `floor-debt` left two LDAP attacks owed, with 0 red: the
`changes` and `search` attacks under `tests/hostile/core/Ldap-Connection/`. Samba's LDAP workers were
crashing with signal 11, and Samba waited up to 120 s before it restarted one. So a `changes` call
timed out on every run. `tests/db/compose.yaml` now sets Samba's restart back-off to one second.
With the container recreated, both attacks pass alone in about 2 s, and both atoms are recorded
green. The next `floor-debt` pays the owed tests and steps that are left. The last run paid such a
round in 78 s.

Expected red: none.

### Next

**Stage 4, the gate.**

- [x] Fix the `questions` entries, then rewrite and stamp each stale or owed page the check names. `website/src/data/questions.json:963`
- [x] Run `looper verify`, fix every failure in one pass, and run it again until green. `docs/goals/first-release.md:227`
- [x] The goal-end gates of AGENTS.md § *In the loop*. `AGENTS.md:1`

### Backlog

- Nothing. The goal's *After the goal* steps are the user's.

