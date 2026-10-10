`bun nv gen-attribution --check` regenerates the third-party notice and compares it,
failing when the lockfile has moved and the notice has not. It runs beside the dependency-policy
job, which it completes: one decides what may be linked, this decides what must be shipped. The two
fail differently. A license the policy does not allow fails loudly the moment its dependency is added;
a missing notice fails silently, and only at distribution, unless this job catches it.

Committing a generated file is deliberate. It makes the notice reviewable in a diff at the moment a
dependency changes, and it keeps the build from depending on network access or on fetched sources.

**Declined:**
- **Generating the notice at build time instead of committing it.** Every build would depend on the
  generator and on fetched sources, and the reviewable diff, which is how a human notices a new
  component at all, would be gone.
