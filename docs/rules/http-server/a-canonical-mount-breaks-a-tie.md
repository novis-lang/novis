A link to another module (`rule:routing/a-link-reaches-another-module`) goes through one row of the
mount table. The candidates for a handler are the rows whose own compiled table holds that handler as
an ordinary route, and the first step with a candidate decides:

1. Rows from the same `[[server.mount]]` block as the row being linked from. A `scan` block counts as
   one block, so `/base/blog` links to `/base/shop`.
2. Otherwise, rows on the same host: the same resolved `host`, or neither row has one. The link is a
   path.
3. Otherwise, rows on another host that resolve an origin. The link is absolute with that row's
   origin, as `urlAbsolute` builds it.

```toml
[[server.mount]]
scan   = "*/public/index.nvs"
prefix = "/base/{1}"

[[server.mount]]
prefix    = "/shop"
entry     = "Shop/public/index.nvs"
canonical = true                      # the main way in to this entry
```

**Two candidates at one step tie, and the row from a `canonical = true` block wins.** With none, or
with two, the boot fails naming both blocks and the handler, the way an unreached origin fails it
(`rule:routing/an-origin-is-per-mount-and-checked-at-boot`). A reload leaves the linking row out
and logs the reason. **A tie no link needs is not an error**, so a table with two mounts of one entry
keeps starting. Two `canonical = true` blocks that mount one entry are `E0621` whether or not a link
needs them: `nvs config check` finds two that write the file, and the expansion finds one a `scan`
reaches.

`canonical` says which door is the main one, which is routing and not policy, so it is in the closed
key set of `rule:http-server/a-mount-carries-no-policy`. It is `System` and reloadable, like `origin`.
`nvs config check` compiles each row's entry through the compile cache, as the boot does, and reports
an unbroken tie a link needs and a link no row serves. `nvs config check --mounts` prints each row,
its block, `canonical`, and the mount each linked handler resolves to.

One function decides, and the boot, the reload, `nvs config check` and `Core\Test::request` all call
it, so the four agree. It reads only the rows, and costs one pass over them per linked handler, off
the request path.

**Declined:**
- **The first block written wins a tie.** Reordering two blocks would then move every link on the
  site, and nothing in the file would say so.
- **A tie is an error whether or not a link needs it.** Existing tables with two mounts of one entry
  would stop starting for a feature they do not use.
