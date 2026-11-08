- **`bun nv proofs --record-perf` measures one `--only` id, and two runs at once can record `0.0`
  ns/op.** A repeated `--only` keeps the last id, so four stale members need four calls. Two calls
  started side by side skew each other's baseline, and one recorded `"ns_per_op": 0.0` against a
  previous 13,116. Run the calls one after another, and compare each new record with the one before
  it before committing `data/perf/members.ndjson`. See `tools/nv/proofs/perf.ts`.
