- **A nightly CI failure in the sweep can be days behind local `main`, because the nightly runs
  `origin/main` and the loop never pushes.** Most of a long list is then fixed already. Run
  `gh run view <id> --json headSha`, compare it with `git rev-parse HEAD`, and re-run each failed step
  at HEAD before changing anything. See `tools/nv/cmd/sweep.ts`. `until: gone tools/nv/cmd/sweep.ts:readNightly`
