- **`bun nv proofs --record-perf` measures one `--only` id per call.** A comma list is read as one
  id that is not on the roster, and a repeated `--only` keeps the last one alone, so three stale
  members take three calls. A `0.0` ns/op is that bench's usual figure where the earlier record
  says so too. See `tools/nv/cmd/proofs.ts`. `until: gone tools/nv/cmd/proofs.ts:--record-perf`
