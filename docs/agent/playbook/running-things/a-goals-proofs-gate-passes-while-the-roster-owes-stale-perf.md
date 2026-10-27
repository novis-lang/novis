- **A goal's `proofs --gate --only <ids>` check passes while the whole roster owes stale perf.** A
  change to a shared file such as `router.rs`, `reflect.rs` or a reference chapter makes every
  feature measured from it stale, far beyond the ids the goal names, and CI's `proofs` job and
  `ci --push` read the whole roster. Run `bun nv proofs --gate --limit 0` and `bun nv site --check
  stale questions` before a goal's end, and record or restamp everything they name. A bench with a
  `.nvsr` that carries a signed token exits 1 once the signing input changes; sign it again with
  `urlSigned` and the bench's own key. See `tools/nv/cmd/proofs.ts`. `until: gone tools/nv/cmd/proofs.ts:--limit`
