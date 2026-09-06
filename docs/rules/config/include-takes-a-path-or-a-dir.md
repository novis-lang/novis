`[[include]]` is a top-level array of tables, and an entry carries `path` **or** `dir`, never both and
never neither:

```toml
[[include]]
path = "conf.d/production.toml"

[[include]]
dir = "conf.d"                    # every *.toml directly inside, ascending by filename

[[include]]
path = "/etc/nvs/local.toml"
optional = true                   # absent is fine
```

`dir` is **not** a glob. It reads every `*.toml` directly inside that directory, in ascending byte
order of filename, without recursing, and the order is mandated rather than inherited from `readdir`
because `rule:config/later-wins-and-every-override-is-recorded` makes order decide the answer — a
capability grant settled by directory-entry order is not a design. A pattern language is refused for
the same reason INI was: a grammar to specify, fuzz and diagnose for a feature whose whole job is
"read this directory".

An `[[include]]` inside an included file is ordinary
(`rule:config/an-include-cycle-is-refused-and-nesting-is-capped-at-eight`), and an included file is
a configuration file in every other respect: unknown and duplicate keys are refused inside it exactly
as in the root, its paths resolve against its own directory, and it must pass
`rule:config/ownership-is-the-trust-boundary`. What `optional` does and does not cover is
`rule:config/optional-covers-absence-and-moves-the-check-to-the-directory`.
