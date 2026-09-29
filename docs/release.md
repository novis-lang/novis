# Cutting a release

A release is one click in the Actions tab and one click on a draft, with a review in between. The
schedule is [.github/workflows/release.yml](../.github/workflows/release.yml) and every decision
it makes is `bun nv release`, in [tools/nv/cmd/release.ts](../tools/nv/cmd/release.ts) — that split is the workflow header's subject
and is not repeated here. **This file is the procedure and the one-time GitHub setup it needs.**

Nothing here is automatic. No push, no schedule and no agent may start a release; the workflow has
exactly one trigger, and it is a person choosing `Run workflow`.

## What happens, in order

| | | Writes |
|---|---|---|
| 1 | `plan` — the next version, and the notes rendered from the commit log | nothing |
| 2 | `test` — the whole of [ci.yml](../.github/workflows/ci.yml), called, not copied | nothing |
| 3 | `build` — seven targets, each at the version being released | nothing |
| 4 | `publish` — bump, changelog, commit, tag, push, draft release | **everything, at once** |
| 5 | `docker` — two image variants, from the binaries step 3 built | the immutable image tags |

Then, on the click that publishes the draft, [release-promote.yml](../.github/workflows/release-promote.yml)
moves `latest` and the `MAJOR.MINOR` image tags. It rebuilds nothing — it re-points a tag at a
digest that already exists — and [docs/docker.md](docker.md) is the user-facing half of all this.

Steps 1–3 write nothing to the repository on purpose: a build that fails after the tag was pushed
would leave a version commit and a tag on `main` for a release that does not exist, and both would
have to be undone by hand on `main`. Nothing is written until every test and every
binary is green.

## Before the first release: setting up GitHub

Six things, once. **None of them is a secret** — see *Credentials* below.

### 1. The code is on `main`, and `landing` is the default branch

`origin` is `github.com/novis-lang/novis`. Its default branch is `landing`, an unrelated branch with
a short README and the files GitHub reads only from the default branch: the issue forms, the
security policy, the Dependabot configuration and one file per workflow. `landing`'s `ci.yml` has
the nightly `schedule` and dispatches CI on `main`. Its other workflow files exist so the **Run
workflow** button appears, and a run started on `landing` stops with an error.

Set `landing` in **Settings → General → Default branch**. Cut a release from the Actions tab with
**Use workflow from → `main`**. The `plan` job refuses to run on any other ref, so a release cut from
a topic branch or from `landing` is impossible rather than discouraged. Under **Settings →
Environments → `github-pages`**, the deployment branches must include `main`.

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

### 5. Public repository, or three build legs stop working

Two matrix legs use GitHub's ARM runners (`ubuntu-22.04-arm`, `windows-11-arm`), which are free on
public repositories and unavailable on private ones. Build provenance attestation is likewise a
public-repository feature at the free tier.

If the repository is private at first release, delete the two `aarch64` legs and the
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

### 7. Nothing else

There is no secret to create, no PAT to store, no deploy key, no signing key and no registry
token — the container registry included. If a future step asks you to add one, that step is the
thing to question.

## Cutting a release

1. **Preview it locally first.** No runner, no credentials, writes nothing:

   ```sh
   bun nv release --preview patch
   ```

   That prints the exact version and the exact notes the run will produce.

2. **Pause the loop.** `publish` takes the current tip of `main` and pushes onto it; if the
   unattended loop commits in the same few seconds, the push is rejected and the run fails after
   the builds. Nothing is corrupted — but the whole matrix has to run again.

3. **Actions → Release → Run workflow.** Choose the branch `main` and:

   | Input | |
   |---|---|
   | `bump` | `patch`, `minor` or `major` — read *The version scheme* below, it is not plain SemVer |
   | `version` | leave empty; an exact version here overrides the arithmetic |
   | `allow_contract` | only when deliberately reaching 0.1.0 |
   | `dry_run` | **tick this for the first ever run** — everything happens except the writing |

4. **Approve the `release` environment** when the run pauses, after checking the notes in the run
   summary and that every build leg passed.

5. **Review the draft** at *Releases*, then press **Publish**. The workflow never does: everything
   it produces is a draft, and announcing it is a separate human decision.

6. That click starts [release-promote.yml](../.github/workflows/release-promote.yml), which points
   the `latest` and `MAJOR.MINOR` container tags at the digest already published under the version
   tag. Nothing is rebuilt. If the `docker` job had to be re-run by hand and the promotion was
   therefore missed, dispatch that workflow with the tag — it is idempotent.

## The version scheme is not plain SemVer

`rule:packaging/below-1-0-the-breaking-slot-moves-left` owns it, and below 1.0 the
breaking slot moves left by one — `0.MINOR` carries breaking changes and `0.MINOR.PATCH` is always
compatible. From today's 0.0.1:

| `bump` | Result | |
|---|---|---|
| `patch` | 0.0.2 | |
| `minor` | 0.0.2 | the same, deliberately: below 1.0 there is no third slot for a compatible feature |
| `major` | 0.1.0 | **and refuses unless `allow_contract` is ticked** |

That refusal is not a formality. `rule:packaging/the-version-contract-starts-at-0-1-0` makes 0.1.0 the release that ends the prototyping
regime and declares the version contract — "the switch is thrown once, in the commit that tags
0.1.0". A dropdown nobody read is not a way to throw it.

## Credentials

The only credential is the per-run `GITHUB_TOKEN`, minted by GitHub for the run and revoked when
it ends. `bun nv release` has no code path that reads a token at all, which is what makes the
whole release reproducible on a laptop with no access to anything.

Three habits in the workflow keep it that way, each load-bearing rather than decorative:
`permissions: {}` at the top with per-job re-grants so only `publish` can write;
`persist-credentials: false` on every checkout but that one, so the token is never written into
`.git/config` where every build script and proc macro in the dependency tree could read it; and
**no `${{ }}` inside any `run:` block**, because an expression is pasted in before the shell parses
it and the commit log this workflow reads is written by an unattended loop, not curated by hand.

## Verifying a published binary

Each archive carries a signed statement that this repository's workflow, at a named commit,
produced those exact bytes — keyless, so there is no key to leak:

```sh
gh attestation verify nvs-0.1.0-linux-x86_64.tar.gz --repo novis-lang/novis
sha256sum --check --ignore-missing SHA256SUMS
```

Archives are deliberately **not** stripped. `[profile.release]` in [Cargo.toml](../Cargo.toml)
keeps `debug = "line-tables-only"` so a production backtrace names lines; stripping the shipped
binary would spend exactly what that setting buys, to save bytes — which AGENTS.md's priority
ordering puts last.

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
| An `optional` build leg failed | `linux-x86_64-musl` and `windows-aarch64` are marked optional because `wasmtime` and `corosensei` carry assembly and neither lists Windows on ARM64 as supported. The release still ships; drop `optional:` from that leg once a run has proved it. |
| A **required** leg failed | `publish` refuses by name rather than shipping a quietly incomplete release. Fix and re-run; nothing was written. |
| The push in `publish` was rejected | `main` moved (see step 2) or is protected (see setup § 4). Nothing was written — re-run. |
| A release went out wrong | The draft is a draft. Delete it, delete the tag, revert the one `chore(release)` commit. **Unless `docker` ran** — a pushed image tag is the one write here that force-pushing a branch does not undo, and § *Recalling a published image* above is that procedure. Do it in that order: deleting the git tag is what unblocks moving `latest` back. |
| `docker pull` says *denied* or asks for a login | The package is still private. Setup § 6 — it is a one-time switch and it is not the repository's own visibility. |
| The `docker` job failed after the release was tagged | Re-run that job alone; it needs nothing from the earlier jobs but their artifacts, and `fail-fast: false` means a variant that already succeeded is not redone. It names the same tags, but the re-run's **digest differs** — the image config records a build time — so the abandoned attempt is left as an untagged version in the package. Then dispatch `release-promote.yml` with the tag, since the publish click has already been and gone. |
| The `docker` job failed *before* the release was tagged | It cannot: it `needs: [plan, publish]`. There is no state where an image exists for a version the repository has no tag for. |
| `latest` did not move | It moves on *publish*, not on draft (see step 6 above), and `nv release` refuses to move it backwards onto a version older than the newest tag. Dispatch `release-promote.yml` to retry. |
