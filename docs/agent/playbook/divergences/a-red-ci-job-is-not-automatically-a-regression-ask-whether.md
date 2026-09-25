- **A red CI job is not automatically a regression — ask whether it has *ever* been green.** A leg
  failing since the commit that introduced it reads as "something recent broke this" and buys a
  confident wrong suspect, and on a short CI history `gh run list` cannot answer the question at
  all. `git log -S "<a flag only that job passes>" -- .github/workflows/ci.yml` finds the commit
  that added the job; run the failing test there first, not last. [until: gone .github/workflows/ci.yml]
