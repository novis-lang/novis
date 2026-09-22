---
milestone: post-parity
position: last
---
# Loop goal 178 — CI is green on `main`, for the commit the run stands on

M0's acceptance is "green on all three platforms in CI", and the closure goals added the extension,
database-matrix and ThreadSanitizer legs. Once this goal is green, the latest `ci.yml` run on `main`
completed with `success` **and ran the commit `main` points at**, so everything the run built is
proven on the hosted runners and not only on this machine — macOS above all, which has no local
stand-in.

## Why here

**Last on the chain, behind every generated goal, by the user's decision of 2026-09-18.** This was
goal `gap-zero`'s stage 4, and it held the run: GitHub starts no job on this private repository while
the account's payments fail, and nothing in the tree changes that. A check that cannot go green is
not traded or rewritten to pass, so it moved — whole — to the one place where holding costs nothing,
which is after the last piece of work that does not need it.

`position: last` in this file's front matter is what keeps it there. `python tools/dossier.py
--emit-goals` appends behind the chain's last goal, so it moves a goal carrying that key back to the
end of what it appended; `python tools/chain.py --new --end` lands in front of one, and `--check`
fails when one is not last.

**What this spends:** every commit landed while CI is blocked is unproven on the hosted runners. The
floor covers Windows natively and Linux under WSL, ThreadSanitizer, the fuzz targets and valgrind
included; macOS is covered by nothing until this goal is reached.

## Stage 0 — the catch-up

None.

## Stage 1 — the floor

The last generated goal's whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never
traded.

## Stage 2 — the run exists, and it is this commit's

`python tools/ci-green.py` must say the latest `ci.yml` run on `main` completed with `success` and ran
the code `HEAD` holds: the run's commit is an ancestor of `HEAD`, and nothing has changed since but
the bookkeeping a session writes — that tool's module doc is the home of which paths those are, and
of why equality with `HEAD` is not what it asks. The loop never pushes, so such a run exists only
after the user pushes `main`; when `origin/main` is behind, that is a `BLOCKED` asking for the push,
and it is the expected first answer of this goal.

If GitHub still starts no job — the run ends `failure` in seconds with zero steps and the annotation
names billing — that is a `BLOCKED` naming the billing block.

## Stage 3 — a red leg is ordinary work

The first run that really executes covers every commit landed since the block began, so a red leg is
likely and is not a second `BLOCKED`: read the failing job's log with `gh run view --log-failed`, fix
what it names, and ask for the push again. `.github/workflows/ci.yml` is the legs; a leg is never
deleted, skipped or marked `continue-on-error` to get a green run.

## Standing decisions

- **The check is never rewritten to pass.** Not the `want`, not the workflow, not the branch.
- **A session never pushes.** `git push` is the user's; the `BLOCKED` names the commit to push.
- **Billing, and whether the repository becomes public or gets a self-hosted runner, are the
  user's.** A session that finds CI blocked says so and stops.
- **A failure on a hosted runner that does not reproduce locally is still a failure.** It is fixed
  from the log, and a platform-specific `cfg` that hides the symptom is not a fix.
