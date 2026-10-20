- **A proof that passes can stay owed forever when its recording run on the covws debug build
  ends differently, and `floor-debt` then reports one atom left with 0 red.** The recording run is many
  times slower, so a served proof near the default 30 s request CPU limit is green on release and
  `FATAL: the request exceeded its CPU-time limit` on covws. `bun nv proofs --id '<feature>' --run`
  prints the `diverged` line with both exit codes. Shrink the work the attack repeats until both runs
  agree; never raise the limit in the proof's `nvs.toml`. See `tools/nv/proofs/run.ts`.
  `until: gone tools/nv/proofs/run.ts:on the recording run`
