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

CI builds no release binaries. The dev build does, after CI: `dev-build.yml` starts when a push's CI
run on `main` finishes, and builds that run's commit through the release's own files.
`build.yml` holds the release binaries and the VS Code extension, and `image.yml` the container
images; the dev build and `release.yml` both call them. Every finished run gets its binaries as the
dev build's artifacts, and a green run also gets the `dev` image. A push that changed only paths no
part of the build reads, such as a goal's handoff or a rule, builds nothing: the dev build diffs the
commit against the newest dev build that still has its artifacts, and those artifacts and the `dev`
image stay current. `UNBUILT` in `tools/nv/cmd/ci-changes.ts` names those paths, and a path it does
not name is built. What that spends: six cold release builds per push that changed a build input,
after CI and outside its job slots and cache, and a `dev` image the next green build replaces, whose
earlier versions stay in the package untagged.

The lanes spend nothing from security, latency or simplicity of the language. The dependency policy
still gates every change that can alter the dependency set, and the sanitizer still gates every
change to a crate that holds unsafe code. What they cost is one table to read before a job is added,
and a job the nightly shows the push lane should have run is one entry in that table.

**Declined:**
- **A second workflow holding the heavy jobs.** It needs a second copy of the toolchain and cache
  setup, and the release gate would be put together from two files that drift apart.
- **Building the release binaries inside the push's CI run.** No CI job compiles what the release
  build does. `cost-guards` builds a release `nvs`, but on another runner image, for the host target
  and without `cargo auditable`, so no compile is saved. The six legs would compete with the tests
  for the account's runners, macOS above all, and a push's verdict would arrive later. The gain is
  binaries about fifteen minutes sooner, and only the `dev` image and its Discord post read them.
- **Deleting the jobs a push defers.** Miri and the fuzzers are the memory-safety evidence a
  JIT-compiled runtime has to keep producing. The question is only how often they run.
