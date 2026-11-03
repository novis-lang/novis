One workflow file holds every CI job, and a schedule trigger plus a `full` flag decide which of them
a given run needs.

The **push lane** covers a push to `main` and every pull request; each job runs under
its own `if:`. The **deep lane** — the nightly schedule, a manual dispatch, and the release gate
calling this same workflow — forces every gate true, so every job runs. Always-on are the document
checks, which cost seconds.

A job is never defined twice. A second copy of a suite drifts, and it drifts silently because
nothing compares the copies, which is why the release gate calls this workflow rather than restating
it — and why a fast lane can never be bought by deleting a job, since that deletes it from the
release gate in the same edit.

Nothing has left the suite. What a push skips is work whose answer its own diff cannot change.

A push to `main` diffs against the newest commit on `main` whose `ci.yml` run went green — a push,
nightly or dispatched run, never a pull request's — and not against the push's `before`. That base is
sound by induction: a full run checked every lane at its commit, and a green push run checked every
lane its diff from a sound base touched, while each lane it did not touch has the same files as at
that base. So a newer push may cancel an older push's run still going, and `bun nv ci --dispatch
dry-run` may cancel the push runs of the commit its release run tests: the next run covers every
commit a cancelled run would have. `bun nv ci-changes --since-green` makes the lookup. When GitHub
cannot list the runs, or none is an ancestor of the commit, no base is known and every lane runs
(`rule:testing/unknown-base-runs-everything`). The push's `before` is not a fallback: its run may have
been cancelled, and a run green from it would become the next push's base without having proved the
commits in between. What that spends: one `gh api` call per push run, and a commit whose run was
cancelled has no verdict of its own, so a bisect between two green runs is local.

CI builds no release binaries. The dev build does, beside CI: `dev-build.yml` starts at each push
to `main` and builds that commit through the release's own files, so its binaries arrive while CI
still runs. `build.yml` holds the release binaries and the VS Code extension, and `image.yml` the
container images; the dev build and `release.yml` both call them. Every push's binaries are the
artifacts of its own dev build run, and a newer push cancels an older push's build that is still
going. When a push's CI run finishes green, a second run of `dev-build.yml` waits for that build and
publishes the `dev` image from its Linux archives, so the image never names a commit CI failed. A
push that changed only paths no part of the build reads, such as a goal's handoff or a rule, builds
nothing: the dev build diffs the commit against the newest green dev build that still has its
artifacts, and those artifacts and the `dev` image stay current. A push whose `CI` run or dev build
was cancelled gets no `dev` image: a newer push's green run publishes the next one. `UNBUILT` in
`tools/nv/cmd/ci-changes.ts` names those paths, and a path it does not name is built. What that
spends: six cold release builds per push that changed a build input, outside CI's cache but beside
its jobs on the account's runners, macOS above all, so a busy hour can slow CI's verdict; a build
for a commit CI then fails; and a `dev` image the next green build replaces, whose earlier versions
stay in the package untagged.

The lanes spend nothing from security, latency or simplicity of the language. The dependency policy
still gates every change that can alter the dependency set, and the sanitizer still gates every
change to a crate that holds unsafe code. What they cost is one table to read before a job is added,
and a job the nightly shows the push lane should have run is one entry in that table.

**Declined:**
- **A second workflow holding the heavy jobs.** It needs a second copy of the toolchain and cache
  setup, and the release gate would be put together from two files that drift apart.
- **Building the release binaries inside the push's CI run.** No CI job compiles what the release
  build does. `cost-guards` builds a release `nvs`, but on another runner image, for the host target
  and without `cargo auditable`, so no compile is saved. `release.yml` calls `ci.yml` as its gate and
  builds through `build.yml` beside it, so a release would build every leg twice. A push's CI verdict
  would also wait for its slowest build leg. The dev build's own run starts at the same push, so its
  binaries arrive as early.
- **Building the binaries once the push's CI run ends.** A tester then waits for CI and for the build
  one after the other. Only the `dev` image needs CI's verdict, and it still waits for a green run.
- **Deleting the jobs a push defers.** Miri and the fuzzers are the memory-safety evidence a
  JIT-compiled runtime has to keep producing. The question is only how often they run.
