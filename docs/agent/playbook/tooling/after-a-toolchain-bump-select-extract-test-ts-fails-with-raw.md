- **After a toolchain bump, `select-extract.test.ts` fails with "raw profile version mismatch".** The
  test runs the `target/covws` `nvs` that is already there, built by the old rustc, and the new
  toolchain's `llvm-profdata` cannot read its profiles. Rebuild the tree with `buildCovws()` from
  `tools/nv/lib/covws.ts` before you verify.
  `until: gone tools/nv/test/select-extract.test.ts:a recorded run of the covws nvs`
