The server configuration is a TOML file named `nvs.toml`, read into the directive registry through the
`toml` crate and `serde` — at boot, and again on each reload, which parses and validates a whole
replacement before publishing it and leaves the running configuration untouched if any part fails.
Pure Rust, no C, and no new dependency class: Cargo's own manifests are TOML.

INI does not survive an argument. It has no specification — every parser
disagrees on comment markers, quoting, nesting and what a duplicate key means — and it has one value
type, string, which is the defect the language itself rejects. TOML has a boolean, an integer, an
array and an array of tables, and an editor already validates it.

`nvs.toml` names the **root of a tree**, not the whole configuration:
`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` says where that root is found
and `rule:config/include-takes-a-path-or-a-dir` how it pulls in more. Every file in the tree is written
in this syntax, and `rule:config/every-block-is-argued-where-it-is-added` lists the blocks.

The `[limits]`/`[limits.hard]` layout is unchanged from the changeability model; only its spelling
moved.
