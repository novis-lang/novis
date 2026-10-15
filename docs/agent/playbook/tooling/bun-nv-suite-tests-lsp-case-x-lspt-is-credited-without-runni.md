- **`bun nv suite tests/lsp/ --case x.lspt` is credited without running the case.** `nvs test` runs
  `.nvst` cases only, and the verdict used to check a named case for being on disk and not skipped,
  never for being one the tree ran. Run `.lspt` cases with `bun nv suite lsp-test tests/lsp/ --case …`,
  which runs `nvs lsp-test` and judges its summary line. A named case the suite does not run is now red
  (`tools/nv/accept/runner.ts`'s `suiteVerdict`). `until: gone tools/nv/accept/runner.ts:is not a case this suite runs`
