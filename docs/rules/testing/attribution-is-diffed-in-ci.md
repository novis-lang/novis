`bun nv gen-attribution --check` regenerates the third-party notice and compares it,
failing when the lockfile has moved and the notice has not. It runs beside the dependency-policy
job, which it completes: one decides what may be linked, this decides what must be shipped.

Committing a generated file is deliberate. It makes the notice reviewable in a diff at the moment a
dependency changes, and it keeps the build from depending on network access or on Python.
