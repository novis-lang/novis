- **`xargs` drops the backslash of a `Core\…` key, so `site --stamp` fails on it.** A list of stale
  keys piped through plain `xargs bun nv site --stamp` turned `help:Core\Config::set` into
  `help:CoreConfig::set`, and the whole stamp failed with "no roster feature has this about.md or
  help card". Use `xargs -d '\n'`, which reads each line as it is. See `tools/nv/cmd/site.ts`.
