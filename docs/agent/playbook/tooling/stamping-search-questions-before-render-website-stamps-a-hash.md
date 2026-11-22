- **A search-question stamp hashes the rendered `core.json`, so stamping before `bun nv render
  --website` stamps a hash that goes stale at the next render.** A stage stamped its pages while
  `core.json` was stale. The gate stage's `close` rendered it, and 57 pages went stale again. The
  stale guard names only 20 of them per run. Render first (`bun nv rules --render`, then
  `NVS_BIN=target/debug/nvs.exe bun nv render --website`), then list every stale page with `bun nv
  site --questions --owed core --limit 0` and stamp them all in one call. See
  `tools/nv/cmd/site.ts` `questionSources`.
