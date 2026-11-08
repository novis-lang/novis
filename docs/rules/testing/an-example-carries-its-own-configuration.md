An example that needs configuration carries it in an `nvs.toml` in its own directory, and the website
shows that file once, whole, above the feature's examples. The repository root's `nvs.toml`
configures nothing under `docs/examples/` or `website/snippets/`: no `[[app]]` there has a `root` or
`entry` under either tree, and no line of it, a comment included, names a path under either tree.
`tools/nv/test/root-config-names-no-example.test.ts` fails on each such block and line.

The reason is the reader who copies an example. The page shows the program, its output and the file it
runs with. A setting the root wrote for it would be a setting the page cannot show, and the copied
program would print something else. A root comment that names an example is how the root says a table
is there for that example to print, so the test reads comments too.

The root keeps every table something outside the two trees reads: the attacks under `tests/hostile/`,
the benches, the tests and `examples/`. A directory whose examples need different grants writes the
shared part as top-level tables and an `[[app]] entry` block only for the example that differs. The
mechanics, and how `--bless` and the sweep run a directory under its file alone, are
`docs/examples/README.md` § *A grant the examples need*.

**Declined:**
- **The same rule for `benches/members/` and `tests/hostile/`.** The website shows no bench and no
  attack source, only their counts and two links, so no reader copies one from a page. The root's
  `root = "tests/hostile"` block also narrows every attack's defaults at once, which a file per
  directory would repeat. Both trees already read a directory's own `nvs.toml`, so a move stays open.
- **Running every example that has no `nvs.toml` under the shipped defaults instead of the root file.**
  It would also catch an example that leans on a root table nothing names. It re-checks every `.out`
  in the tree, and a snippet served under the root needs the root's `[queue]` on PostgreSQL, so it
  waits until the root file has nothing an example could lean on.
