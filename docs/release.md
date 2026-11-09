# Cutting a release

A release is one click in the Actions tab and one click on a draft, with a review in between. The
schedule is [.github/workflows/release.yml](../.github/workflows/release.yml) and every decision
it makes is `bun nv release`, in [tools/nv/cmd/release.ts](../tools/nv/cmd/release.ts) — that split is the workflow header's subject
and is not repeated here. **This file is the procedure and the one-time GitHub setup it needs.**

Nothing here is automatic. No push and no schedule starts a release; the workflow has exactly one
trigger, `workflow_dispatch`. A **dry run** writes nothing, tags nothing and pushes no image, and any
agent may dispatch one with `bun nv ci --dispatch dry-run`, whose single argv always carries
`dry_run=true`, and read its verdict with `bun nv ci --release-status`. The workflow's `run-name`
titles such a run `Release (dry run)` in the Actions list.

A **real run** is a person's decision. A person cuts it from the Actions tab, or an agent cuts it
through `bun nv ci`, only when the live goal under `docs/goals/` says to cut a release, and only with
the commands § *Cutting a release* names. Those commands refuse a real run without a green dry run of
`main`'s tip, approve only a run whose every build passed, and publish only a whole draft. No
command cancels a `Release` run.

## What happens, in order

| | | Writes |
|---|---|---|
| 1 | `plan` — the next version, and the notes rendered from the commit log since the newest `v*` tag | nothing |
| 2 | `test` — the whole of [ci.yml](../.github/workflows/ci.yml), called, not copied; skipped when the commit already passed a full CI run on `main` (the nightly or a `workflow_dispatch`) or a green `Release` run, such as the dry run before a real one | nothing |
| 3 | `build` — the release targets and the VS Code extension as `nvs-<version>.vsix`, each at the version being released, from [build.yml](../.github/workflows/build.yml); it runs beside `test`, not after it | nothing |
| 4 | `publish` — once `test` and `build` both passed: bump, changelog, commit, tag, push, draft release with the archives and the `.vsix` | **everything, at once** |
| 5 | `docker` — two image variants, from the binaries step 3 built, each smoke-tested by `tests/docker/smoke.sh` before it is pushed, from [image.yml](../.github/workflows/image.yml) | the immutable image tags; in a dry run, nothing |

Then, on the click that publishes the draft, [release-promote.yml](../.github/workflows/release-promote.yml)
moves `latest` and the `MAJOR.MINOR` image tags, then deploys the website again so it shows the new
version. It rebuilds no image — it re-points a tag at a
digest that already exists — and the website's Docker guide,
[docker.mdx](../website/src/content/docs/guides/production/docker.mdx), is the user-facing half of
all this.

Steps 1–3 write nothing to the repository on purpose: a build that fails after the tag was pushed
would leave a version commit and a tag on `main` for a release that does not exist, and both would
have to be undone by hand on `main`. Nothing is written until every test and every
binary is green.

### What the version commit changes

`publish` runs `bun nv release --apply` and commits exactly the files it writes, by name:
`Cargo.toml` (the workspace version and every path dependency's pin), `Cargo.lock`,
`fuzz/Cargo.lock`, `CHANGELOG.md`, and `editors/vscode/package.json` with its `package-lock.json`.
`fuzz/` is a separate cargo workspace that uses workspace crates by path, so its lockfile pins their
version too. Each lockfile gets an offline update of the workspace crates alone, so a release moves no
third-party dependency, and each job that runs `--apply` first fetches every committed lock it
updates. The header of [release.ts](../tools/nv/cmd/release.ts) is the home of these rules, and
`SEPARATE_LOCKS` there lists the lockfiles beside the root one.

No rendered file carries the version, so a release leaves none behind: the website's
`/llms-full.txt` prints `Core\Env::VERSION` the same for every build, as it prints the constants
that differ per platform. After the version commit, `main` passes `bun nv ci --check` with no
file rendered again.

## Setting up GitHub

GitHub is set up for releases: § 1 to § 5 are done. They are here for a new fork or a moved
repository, and § 6 is the one step left, after the first release. **None of them is a credential**,
and the release itself reads none — see *Credentials* below.

### 1. `main` is the default branch

`origin` is `github.com/novis-lang/novis`, and its default branch is `main`. GitHub reads several
files only from the default branch: the issue forms, the security policy, the Dependabot
configuration, and the `schedule` trigger. So the nightly in `ci.yml` runs only while `main` is the
default. Every run of [ci.yml](../.github/workflows/ci.yml) builds the release binaries with
`build.yml`, with no version, and its `build` job keeps them as the run's artifacts: those are the dev
binaries. When a push to `main` has a green run, its `dev-image` job publishes the `dev` image from
those binaries with `image.yml`.

Set `main` in **Settings → General → Default branch**. Cut a release from the Actions tab on
`main`. The `plan` job refuses to run on any other ref, so a release cut from a topic branch is
impossible rather than discouraged. Under **Settings → Environments → `github-pages`**, the
deployment branches must include `main`.

`Cargo.toml`'s `repository` is `novis-lang/novis` and its `homepage` is `novis-lang.org`.
`bun nv release` builds commit links from the first of those when it runs outside CI; inside CI the
runner supplies the real one, which is why a stale value there produces correct release notes and
wrong local previews — the worst kind of stale, because nothing fails. Change either only alongside
the repository actually moving.

### 2. Workflow permissions default to read

**Settings → Actions → General → Workflow permissions** → select **Read repository contents and
package permissions**. Leave *Allow GitHub Actions to create and approve pull requests* unchecked.

Both workflows declare the permissions they need per job, so this default is only a floor — but it
is the floor that applies to any workflow anyone adds later without thinking about it.

### 3. The `release` environment — this is the approval gate

**Settings → Environments → New environment**, named exactly `release`. Inside it:

- **Required reviewers** → add yourself. This is what makes the tag and the push wait for a human
  click *after* the binaries are green: the last point at which a release can be called off for
  nothing.
- **Deployment branches and tags** → *Selected branches* → `main`.

Do **not** add any environment secret. The `publish` job needs none.

### 4. If `main` is protected, `publish` needs a bypass

`publish` pushes a commit and a tag to `main` using the run's own `GITHUB_TOKEN`. Branch
protection blocks that like any other push.

- **`main` is unprotected** (the state today — the unattended loop already pushes to it directly):
  nothing to do.
- **`main` has a ruleset**: **Settings → Rules → Rulesets →** your ruleset **→ Bypass list →**
  add the role that the release runs as. Rulesets are the newer system and are the only one with a
  bypass list; classic branch protection has no equivalent, so a classic rule means you must
  either drop it or switch to the GitHub App variant of this workflow.

Note that a push made with `GITHUB_TOKEN` does **not** trigger workflows. That is what stops the
release commit from starting a fresh CI run, and it is deliberate.

### 5. Public repository, or two build legs and the attestation stop working

The `linux-aarch64` matrix leg uses GitHub's ARM runner (`ubuntu-22.04-arm`), which is free on
public repositories and unavailable on private ones. Build provenance attestation is likewise a
public-repository feature at the free tier.

If the repository is private at first release, delete the `linux-aarch64` leg and the
`attest-build-provenance` step, or expect them to fail. Everything else is unaffected.

### 6. After the *first* release: make the container package public

Nothing is needed to *push* the images — `GITHUB_TOKEN` may write packages owned by this
repository's owner, and the package is created and linked to the repository on the first push.

But **a new GHCR package is private even when its repository is public**, so until you change it
once, `docker pull ghcr.io/novis-lang/novis:…` fails with an authentication error for everyone
who is not you. After the first release run:

**Your profile → Packages → `novis` → Package settings → Danger Zone → Change visibility →
Public.** Leave *Inherit access from source repository* on; it is what makes the release workflow
able to push to it without any credential of its own.

This is once, ever. Subsequent releases push to the same package.

### 7. The VS Code Marketplace and Open VSX

`release-promote.yml`'s `marketplace` job lists each published release's `.vsix` on the VS Code
Marketplace and on Open VSX, and it signs in to both with no stored credential:

- **The publisher** `novis-lang` on the Marketplace (registered as `Novis-Lang`; the lookup ignores
  case), owned by the Microsoft account of the Azure DevOps organisation.
- **A Trusted Publishing policy** on that publisher (the publisher's management page →
  *Trusted Publishing*): repository owner `novis-lang`, repository `novis`, workflow
  `release-promote.yml`, environment `marketplace`. `vsce publish --oidc` sends the job's GitHub OIDC
  token, and the Marketplace exchanges it for a token that lasts at most 15 minutes only when all four
  match. There is no Azure identity, no Entra ID credential and no repository secret.
- **Settings → Environments → New environment**, named exactly `marketplace`: **Required reviewers** →
  add yourself, and **Deployment branches and tags** → *Selected branches and tags* → the tag rule
  `v*` and the branch `main`. A release runs the job on its tag, and a repair dispatch on `main`.
- **The namespace** `novis-lang` on open-vsx.org, created with `ovsx create-namespace` and claimed as
  verified. Its owner signed the Eclipse Publisher Agreement with an Eclipse account linked to their
  GitHub account.
- **A trusted publisher** on that namespace (open-vsx.org → *Settings → Trusted Publishers*): owner
  `novis-lang`, repository `novis`, workflow `release-promote.yml`, environment `marketplace`.
  `ovsx publish --trusted-publishing` exchanges the job's OIDC token for a token that lasts five
  minutes. Open VSX registers a trusted publisher only for an extension that has a version already,
  so a new namespace's first version is uploaded by hand with a token from *Settings → Access Tokens*
  (`ovsx publish nvs-<version>.vsix` with `OVSX_PAT` set), and that token is deleted straight after.

### 8. Nothing else

There is no other secret to create, no PAT to store, no deploy key, no signing key and no registry
token — the container registry, the Marketplace and Open VSX included. If a future step asks you to add one,
that step is the thing to question.

## Cutting a release

This is the whole of a real run, the first one included. Step 9 is once, ever. Each step names the
click a person makes and the `bun nv ci` command an agent runs in its place, under a live goal that
says to cut the release. Each command waits for the step before it to be green, and step 9 is a
person's click either way.

1. **The registry's verifier key is pinned.** `DEFAULT_KEY` in
   [manifest.rs](../crates/nvs-cli/src/package/manifest.rs) is the public key of
   `registry.novis-lang.org`, made with `nvs registry key`. Its private key is outside this
   repository, no agent reads it, and it goes only to the registry service. Nothing is done here unless the
   key is replaced. A new key is a code commit, so the release's `test` job runs the full suite on it
   unless a full CI run of it is already green.

2. **Preview it locally.** No runner, no credentials, writes nothing:

   ```sh
   bun nv release --preview patch
   ```

   That prints the exact version and the exact notes the run will produce.

3. **Pause the loop** with `looper loop pause`. `publish` takes the current tip of `main` and pushes
   onto it; if the unattended loop commits in the same few seconds, the push is rejected and the run
   fails after the builds. Nothing is corrupted — but the whole matrix has to run again.

4. **Start the run.** An agent runs `bun nv ci --dispatch release`, then `bun nv ci --watch
   --release`. The command refuses unless the newest completed `Release` run is a green dry run of
   `main`'s tip, and while any `Release` run is going. A person uses **Actions → Release → Run
   workflow**, chooses the branch `main`, and sets:

   | Input | |
   |---|---|
   | `bump` | `patch`, `minor` or `major` — read *The version scheme* below, it is not plain SemVer |
   | `version` | leave empty; an exact version here overrides the arithmetic |
   | `allow_contract` | only when deliberately reaching 0.1.0 |
   | `dry_run` | **leave it unticked** for a real run. A dry run does everything except the writing; `bun nv ci --dispatch dry-run` starts one, and `bun nv ci --release-status` reads it. A green dry run on `main`'s tip is the rehearsal |

5. **Approve the `release` environment** when the run pauses, after checking the notes in the run
   summary and that every build leg passed. `bun nv ci --approve-release` does both checks, prints
   the notes, and then approves. It refuses a dry run, a run not waiting on `release`, and a run
   where a job before `publish` did not pass.

6. **Review the draft** at *Releases*, then press **Publish**. The workflow never does: everything
   it produces is a draft, and announcing it is a separate decision. `bun nv ci --publish-release`
   publishes it only when every job passed, `docker` included, and the draft has each required
   archive, the `.vsix` and `SHA256SUMS`.

7. That click starts [release-promote.yml](../.github/workflows/release-promote.yml), which points
   the `latest` and `MAJOR.MINOR` container tags at the digest already published under the version
   tag. Nothing is rebuilt. Its last step dispatches [pages.yml](../.github/workflows/pages.yml) on
   `main`, so the website shows the new version: the release commit was pushed with the run's
   `GITHUB_TOKEN`, and that push started no workflow. If the `docker` job had to be re-run by hand
   and the promotion was therefore missed, dispatch that workflow with the tag — it is idempotent.

   Its `marketplace` job then waits on the `marketplace` environment. **Approve it** in the run's
   page, and it lists the release's `.vsix` on the VS Code Marketplace and Open VSX. Until then `bun nv ci
   --release-published` stays red and names that approval.

8. **Resume the loop.** First `git fetch` and `git merge --ff-only origin/main`, so local `main`
   has `publish`'s version commit, then `looper loop resume`.

9. **After the first release, make the container package public:** setup § 6, *Packages → `novis`
   → Package settings → Change visibility → Public*. GitHub has no API for this, so it is always a
   person's click.

10. **Check what a user gets.** `bun nv ci --release-published` checks that the release is
    published, its tag is on `main`, its promote run passed, and
    `ghcr.io/novis-lang/novis:<version>` pulls with no login. When only the last part fails, the
    package is still private, and it prints step 9 as the one step left. Then check by hand that
    each archive link on the website's
    [install page](../website/src/content/docs/install/index.mdx) downloads.

## The version scheme is not plain SemVer

`rule:packaging/below-1-0-the-breaking-slot-moves-left` owns it, and the website's
[What changed](../website/src/content/docs/in-depth/what-changed.mdx) page says it to users. What
it means for `bump` is the table below, from `0.0.1`. A release always advances past the newest `v*`
tag, and a clone without its tags stops at `plan` and says to run `git fetch --tags`:

| `bump` | From 0.0.1 | |
|---|---|---|
| `patch` | 0.0.2 | |
| `minor` | 0.0.2 | the same, deliberately: below 1.0 there is no third slot for a compatible feature |
| `major` | 0.1.0 | **and refuses unless `allow_contract` is ticked** |

That refusal is not a formality. `rule:packaging/the-version-contract-starts-at-0-1-0` makes 0.1.0 the release that ends the prototyping
regime and declares the version contract — "the switch is thrown once, in the commit that tags
0.1.0". A dropdown nobody read is not a way to throw it.

## Credentials

The release's only credential is the per-run `GITHUB_TOKEN`, minted by GitHub for the run and
revoked when it ends. `bun nv release` has no code path that reads a token at all, which is what
makes the whole release reproducible on a laptop with no access to anything. The Marketplace and
Open VSX uploads (§ 7) exchange the run's OIDC token for short-lived tokens, so they add no credential.

Three habits in the workflow keep it that way, each load-bearing rather than decorative:
`permissions: {}` at the top with per-job re-grants so only `publish` can write;
`persist-credentials: false` on every checkout but that one, so the token is never written into
`.git/config` where every build script and proc macro in the dependency tree could read it; and
**no `${{ }}` inside any `run:` block**, because an expression is pasted in before the shell parses
it and the commit log this workflow reads is written by an unattended loop, not curated by hand.

## What an archive carries

Each archive carries a keyless build provenance attestation, so there is no key to leak, and the
release carries `SHA256SUMS`. The release also attaches the VS Code extension as `nvs-<version>.vsix`,
attested the same way and listed in `SHA256SUMS`; `publish` refuses a release without it. Its version
is the release's, because `--apply` rewrites `editors/vscode/package.json` and its lockfile with the
workspace. Publishing the release lists that same file on the VS Code Marketplace and Open VSX, once a reviewer
approves `release-promote.yml`'s `marketplace` job
(`rule:ide/the-extension-runs-where-the-binary-is`). How a user checks both is the website's
[install page](../website/src/content/docs/install/index.mdx) § *Check a release archive*.

Archives are deliberately **not** stripped. `[profile.release]` in [Cargo.toml](../Cargo.toml)
keeps `debug = "line-tables-only"` so a production backtrace names lines; stripping the shipped
binary would spend exactly what that setting buys, to save bytes — which AGENTS.md's priority
ordering puts last. A Linux binary's debug sections are zlib-compressed by `bun nv release
--package` instead, which keeps all of it at well under half the size; that function's doc comment says
why the sections are that large and what the first backtrace costs.

## Recalling a published image

A pushed image tag is the one write in this pipeline that reverting a commit does not undo, so it
is the one rollback with a procedure. Three levels — take the lowest one that fixes the problem.

**1. Move the floating tags back to the previous release.** Everyone pulling `latest` or the
`MAJOR.MINOR` line stops getting the bad build; everyone who pinned a version or a digest is
untouched. **Delete the bad `vX.Y.Z` git tag first.** `nv release` refuses to move a floating tag
onto a version older than the newest `v*` tag — that is what stops a stale draft walking `latest`
backwards, and it is also what silently blocks this rollback while the bad tag still exists.
With it gone, dispatch [release-promote.yml](../.github/workflows/release-promote.yml) with the
good tag and both variants move.

**2. Release a fixed patch.** Almost always better than deleting anything: it costs one run, and
it leaves the record of what happened intact.

**3. Delete the package version** — for a leaked secret or a compromised build, not for a bug.
*Packages → `novis` → Versions →* the version *→ Delete*, or

```sh
gh api -X DELETE /orgs/novis-lang/packages/container/novis/versions/<id>
```

Three things to know before you do: a **digest pin stops resolving**, which breaks a deployment
that was doing the most careful thing available; GitHub will not delete any version of a public
package once it passes 5,000 downloads, so past that it is a support request; and a deleted
version can be restored for 30 days afterwards, which is the escape hatch if the deletion itself
was the mistake.

Deleting the *whole* package also unlinks it from the repository, so setup § 6's visibility switch
has to be done again on the next release.

## When it goes wrong

| | |
|---|---|
| `plan` refuses: *crosses into the version contract* | Working as intended. Read § *The version scheme*; tick `allow_contract` only if you mean it. |
| An `optional` build leg failed | `linux-x86_64-musl` is marked optional because it has to build `ring` and `libsqlite3-sys`'s C with `musl-gcc`. The release still ships; drop `optional:` from that leg once a run has proved it. Windows on ARM64 has no leg at all: `corosensei` does not compile for it. |
| A **required** leg failed | `publish` refuses by name rather than shipping a quietly incomplete release. Fix and re-run; nothing was written. |
| The push in `publish` was rejected | `main` moved (see step 3 of § *Cutting a release*) or is protected (see setup § 4). Nothing was written — re-run. |
| A release went out wrong | The draft is a draft. Delete it, delete the tag, revert the one `chore(release)` commit. **Unless `docker` ran** — a pushed image tag is the one write here that force-pushing a branch does not undo, and § *Recalling a published image* above is that procedure. Do it in that order: deleting the git tag is what unblocks moving `latest` back. |
| `docker pull` says *denied* or asks for a login | The package is still private. Setup § 6 — it is a one-time switch and it is not the repository's own visibility. |
| The `docker` job failed after the release was tagged | Re-run that job alone; it needs nothing from the earlier jobs but their artifacts, and `fail-fast: false` means a variant that already succeeded is not redone. It names the same tags, but the re-run's **digest differs** — the image config records a build time — so the abandoned attempt is left as an untagged version in the package. Then dispatch `release-promote.yml` with the tag, since the publish click has already been and gone. |
| The `docker` job failed *before* the release was tagged | Only a dry run gets there, and a dry run pushes nothing, so no image exists for a version the repository has no tag for. Its failure is the smoke test or the image build, and the step log names which check. `tests/docker/smoke.sh` runs the same checks locally — [docker/README.md](../docker/README.md) § *Building an image locally*. |
| `latest` did not move | It moves on *publish*, not on draft (see step 7 of § *Cutting a release*), and `nv release` refuses to move it backwards onto a version older than the newest tag. Dispatch `release-promote.yml` to retry. |
| The Marketplace or Open VSX upload failed | Re-run the promote run's failed job (`gh run rerun <id> --failed`), or dispatch `release-promote.yml` on `main` with the tag, and approve its `marketplace` environment. Neither releases anything again. `--skip-duplicate` makes an upload a success when the version is listed already, so the rerun repairs whichever one failed. A failed token exchange at publish is a Trusted Publishing policy or trusted publisher whose owner, repository, workflow or environment is not exactly this job's (§ 7). |
