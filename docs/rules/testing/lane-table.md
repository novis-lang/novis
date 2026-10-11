The first job compares the push's or the pull request's base against `HEAD` and emits one boolean
per lane; every other job's `if:` is a single comparison against one of them. Build, test,
conformance and lint run when any build input changed; the sanitizer and the release-profile cost
guards when a native crate changed; the dependency checks when the dependency set changed; the
reference check when the binary or its chapters changed; the feature proofs when the binary or any
feature's proofs changed.

The table deciding those booleans is a dictionary in **one file**, `tools/nv/cmd/ci-changes.ts` — not a set of path filters
spread over the workflow, and not a third-party action on the critical path of every run.
`bun nv ci-changes --base HEAD~1` answers "what would CI have run for this commit" without
pushing anything.

**The platform matrix is not one of the gates.** Whenever any build input changed, all three
platform legs run. This is a public repository and nothing here may assume which host a contributor
develops or pushes from, so no platform's coverage is ever traded away by a path rule — the
contributor who never sees a Windows failure until the nightly is precisely the one who cannot
reproduce it.

**Declined:**
- **A third-party path-filter action.** It would sit on the critical path of every run, pinned and
  audited by whoever updates it, to compute a diff this repository's own tools already compute. The
  supply chain is a security question.
- **Narrowing the platform matrix by path**, so a push builds one platform unless its diff looks
  native. It is the largest saving on offer. It rests on a claim nothing enforces, that a change
  outside the native crates cannot change per-platform behaviour, and that claim is false as soon as
  a front-end change alters what gets compiled or a command-line change touches a path, a line ending
  or a file mode.
- **A workflow-level path filter.** It breaks required status checks
  (`rule:testing/a-deferred-job-is-skipped-never-absent`).
