The server configuration is a TOML file named `nvs.toml`, read into the directive registry through the
`toml` crate and `serde` — at boot, and again on each reload, which parses and validates a whole
replacement before publishing it and leaves the running configuration untouched if any part fails.
Pure Rust, no C, and no new dependency class: Cargo's own manifests are TOML.

INI does not survive an argument. It has no specification — every parser
disagrees on comment markers, quoting, nesting and what a duplicate key means — and it has one value
type, string, which is the defect the language itself rejects. TOML has a boolean, an integer, an
array and an array of tables, and an editor already validates it. The configuration was never all
strings: "no ceiling", a list of debug modes, a list of capability roots and an extension with its hash
pin are a boolean, two arrays and a repeated record, and each would otherwise be a mini-grammar to
specify, parse, fuzz and diagnose. With TOML the implementation carries no dialect parser, no comma or
`PATH` splitter and no `on`/`off`/`yes`/`no` table, and a parse error names a line. The familiarity INI
offers is thin: almost no directive shares a key name with `php.ini`, so no PHP file was ever
copy-pastable, and the sectioned key/value shape that does transfer is TOML's too.

`nvs.toml` names the **root of a tree**, not the whole configuration:
`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` says where that root is found
and `rule:config/include-takes-a-path-or-a-dir` how it pulls in more. Every file in the tree is written
in this syntax, and `rule:config/every-block-is-argued-where-it-is-added` lists the blocks.

The `[limits]`/`[limits.hard]` layout is the changeability model's
(`rule:config/three-changeability-classes`); this rule gives it only its syntax.

- **Memory:** nothing on the request path. The parse runs at boot and on each reload, over files
  measured in kilobytes, and never per request or from a request-serving core; what a live snapshot
  holds is `rule:config/the-config-is-an-immutable-snapshot`'s to state.

**Declined:**
- **INI.** No specification, one value type, four directives that would encode non-strings as strings,
  a `:`-joined root list that cannot spell a Windows path, and a dialect Novis would own forever.
- **YAML.** Indentation-significant, with a parser-CVE record and a value whose *type* depends on how it
  was typed (`no` as `false`, a version number as a float), in the root-owned file that grants
  capabilities.
- **JSON or JSONC.** JSON has no comments, in a file whose main job is recording why an operator chose a
  number; JSONC is one non-standard extension, which brings back the missing specification.
- **KDL, RON or HCL.** An operator meets the format for the first time and the editor does not validate
  it, and nothing any of them offers is reachable at this file's size.
- **Dhall, or configuration written in Novis.** Either makes the configuration a program, reopening the
  door `rule:security/closed-doors` closed on `eval` in the one root-owned file read before any sandbox
  exists.
- **`${ENV_VAR}` interpolation in a value.** It makes the configuration compute, the same door Dhall
  reopens. The run mode is never read from the environment
  (`rule:config/no-environment-variable-selects-the-mode`), and an inherited, unaudited namespace is no
  source for the file that grants capabilities. A secret arrives as a file instead
  (`rule:config/a-secret-is-a-file-whose-content-is-the-value`).
- **Accepting both `.ini` and `.toml`.** Two formats to specify, parse and test, and a deployment that
  holds both with no answer for which wins; nothing in Novis gets a second syntax.
