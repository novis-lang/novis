- **Git Bash rewrites a site URL argument into a Windows path.** `bun nv site --questions --stamp
  /reference/cli/nvs-debug/` reaches the tool as `C:/Program Files/Git/reference/...` and fails with
  "no built page has this URL". Prefix the call with `MSYS_NO_PATHCONV=1`. See `tools/nv/cmd/site.ts`.
  `until: gone tools/nv/cmd/site.ts:--stamp`
