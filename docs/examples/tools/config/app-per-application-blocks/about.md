An `[[app]]` block contains the settings for one application: its limits, its capabilities, its
mode and its origin.

A block names the programs that it covers in one of two ways. `root` is a directory, and the block
covers every entry file inside it. `entry` is one file. A block has exactly one of the two. A block
with both or with neither gives the error `E0609`.

When `nvs run <file>` starts, every block that covers the file applies. The global blocks apply
first, then the `root` blocks from the widest directory to the narrowest, then the `entry` block. A
later value replaces an earlier one. So an `[[app]]` block can lower a limit, lower a ceiling, or
remove a capability that the global configuration grants.

**Good to know:** a `root` or `entry` that does not exist stops the run with an error.
`[app.limits]` and `[app.limits.hard]` take the same keys as `[limits]` and `[limits.hard]`.
