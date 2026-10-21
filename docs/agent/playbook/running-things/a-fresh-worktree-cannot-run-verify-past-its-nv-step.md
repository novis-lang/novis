- **A fresh worktree's `bun nv verify` stops at its `nv` step.** Those self-tests need the coverage
  build under `target/covws`, which only the later `build` step makes. To verify one slice apart from
  a shared tree, run `bun install --frozen-lockfile`, copy the untracked `tests/db/ca.crt` in, then
  run clippy, `target/debug/nvs test tests/conformance`, `bun nv reference` and `bun nv soundness
  --check` by hand. See `tools/nv/cmd/verify.ts`. `until: gone tools/nv/cmd/verify.ts:if (selftest !== null && !built) steps.push(selftest);`
