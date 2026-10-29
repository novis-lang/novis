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

A push also builds what a release ships, through the release's own files rather than a copy of
them. `build.yml` holds the release binaries and the VS Code extension, and `image.yml` the container
images; `ci.yml` and `release.yml` both call them. Every push uploads its binaries as the run's
artifacts, and a push to `main` publishes the `dev` image once every other job in `ci.yml` is green.
What that spends: six cold release builds per push, run beside the tests, and a `dev` image the next
green push replaces, whose earlier versions stay in the package untagged.

The lanes spend nothing from security, latency or simplicity of the language. The dependency policy
still gates every change that can alter the dependency set, and the sanitizer still gates every
change to a crate that holds unsafe code. What they cost is one table to read before a job is added,
and a job the nightly shows the push lane should have run is one entry in that table.

**Declined:**
- **A second workflow holding the heavy jobs.** It needs a second copy of the toolchain and cache
  setup, and the release gate would be put together from two files that drift apart.
- **Deleting the jobs a push defers.** Miri and the fuzzers are the memory-safety evidence a
  JIT-compiled runtime has to keep producing. The question is only how often they run.
