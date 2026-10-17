- **A bench the sweep reports red names no cause in the run's prompt.** The finding, such as which
  count grew and by how much, is printed by `tools/nv/cmd/benches.ts`'s `benchTurn` into looper's
  `after_session` hook log, and the prompt keeps only the summary line. Grep that log for `RED
  <bench path>` before ramping the bench again.
  `until: gone tools/nv/cmd/benches.ts:their growth failed`
