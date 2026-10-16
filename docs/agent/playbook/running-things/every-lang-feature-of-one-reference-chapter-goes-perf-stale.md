- **Every `lang:` feature of one reference chapter goes perf-stale when any line of that chapter changes.**
  A language feature's implementing file is its chapter under `docs/reference/lang/`, and the perf record
  hashes the whole file, so adding one section makes every bench of the chapter owe a new figure.
  Record the chapter's group once, after its last edit, with `bun nv proofs --record-perf --group
  lang:<chapter>`, and commit `docs/perf/members.ndjson` with it. `until: gone tools/nv/proofs/roster.ts:chapterFeatures`
