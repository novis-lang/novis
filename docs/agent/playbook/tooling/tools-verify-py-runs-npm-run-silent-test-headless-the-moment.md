- **`tools/verify.py` runs `npm run --silent test:headless` the moment
  `editors/vscode/package.json` exists**, so the manifest, the runner and a green suite land in one
  group or every session's gate goes red. `scripts/headless.mjs` prints a line only for a suite with
  compiled cases under `out/test/<name>/` and exits 1 when there are none, so naming a stage cannot
  fake it green and the loop's `want` list stays red until each suite is real. Add one by creating
  `test/<name>/*.test.ts` — nothing registers it elsewhere.
  [until: gone editors/vscode/scripts/headless.mjs:ORDER]
