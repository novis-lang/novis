- **`nv verify` can be red on a tree you did not touch, and a step that has been red hides every
  step after it.** `git status --short` showing the failing file unmodified is the whole diagnosis,
  and because the gate stops at the first failure, the steps behind a long-red one (the `.nvst`
  trees, clippy, `cargo doc`) have not been passing either. Fix it in its own commit named for what
  it is; for `fmt`, `grep -c "^Diff in" .agent-tmp/verify-fmt.log` is the blast radius — one file
  means fix it here, a dozen means say so in the handoff instead. [until: reviewed 2026-09-06]
