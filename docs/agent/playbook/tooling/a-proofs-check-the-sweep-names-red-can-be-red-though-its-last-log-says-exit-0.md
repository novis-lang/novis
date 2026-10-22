- **A proofs check the sweep names red can be red though its last log says `exit 0`.** A
  `bun nv proofs --gate --only <id>` check goes `perf: stale` as soon as any edit touches the
  member's source file, and the log under `.looper/loops/main/checks/` may be from a run before that
  edit. Run the check's own command once before calling it stale, and re-record with
  `bun nv proofs --record-perf --only '<id>'`. See `tools/nv/proofs/collect.ts`.
  `until: gone tools/nv/proofs/collect.ts:changed since it was last measured`
