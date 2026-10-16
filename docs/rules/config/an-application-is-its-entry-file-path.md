An application's identity is the **path of its entry file**, so `nvs run` on the command line has one
exactly as a served request does. A per-app block is `[[app]]` carrying `root` **or** `entry`, never
both and never neither (`E0609`):

```toml
[[app]]
root = "/srv/www/shop"                     # every entry file beneath this directory
mode = "production"
[app.limits]
memory = "512M"
[app.capabilities]
process.exec = true                        # this application only

[[app]]
entry = "/srv/www/shop/bin/import.nvs"     # one file
[app.limits]
wall_time = "600s"
```

`root` matches when it is a prefix of the entry file's path **on path-component boundaries**, so
`root = "/srv/www/shop"` matches `/srv/www/shop/bin/import.nvs` and not
`/srv/www/shopfront/index.nvs`. `entry` matches one file exactly — the most specific form of the same
test. A block's directives live in `[app.limits]`, `[app.limits.hard]`, `[app.capabilities]` and
`[app.log]` sub-tables, which attach to the preceding `[[app]]` by ordinary TOML rules; `mode` and
`origin` sit directly on the block.

An application is a tree and not a file because one deployment holds `bin/import.nvs`, `bin/migrate.nvs`
and whatever else `nvs run` is pointed at, all wanting the same limits and grants; keying on one file
would make a new script silently inherit the global defaults. Both keys are paths, so
`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` applies, and blocks
accumulate across the tree like every other array of tables. An entry file matched by no block gets
the global configuration, which is the ordinary case.

A sub-table such as `[app.limits]` attaching to the `[[app]]` above it is valid TOML that reads
subtly: a reader who takes `[app.limits]` for a global block is wrong, and nothing in TOML flags it.
The shipped configuration file writes each sub-table directly under its `[[app]]` block and says it
applies to that application alone.

**Declined:**
- **An exact entry-file key only, with no prefix matching.** A project with thirty `bin/*.nvs` scripts
  needs thirty blocks, and the thirty-first silently falls back to the global defaults.
- **A quoted path as the table key**, `[app."/srv/www/shop/index.nvs".limits]`. It avoids the
  array-of-tables subtlety, at the cost of a Windows path needing literal-string quoting inside a table
  key, long unreadable lines, and no room for a selector that is not a path.
- **Deriving an application from the package or `require` graph.** It would identify an application
  more precisely than a directory does, and it needs the compiler to have run before the configuration
  is known. The configuration is read at boot, and programs are resolved after it.
