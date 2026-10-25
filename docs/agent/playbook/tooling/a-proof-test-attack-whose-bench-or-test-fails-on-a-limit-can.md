- **A `proof: test` attack whose bench or test fails on a limit can stay owed forever, with 0 red.**
  Where a stack or memory limit stops it depends on the build, so its covws recording run prints
  other counts and figures and `diverged` repeats on every run. `bun nv proofs --run --id <feature>`
  names the line, and `testReport` in `tools/nv/proofs/run.ts` is where such a line is left out of
  the comparison. `until: gone tools/nv/proofs/run.ts:exceeded its memory limit`
