`nvs config check` checks a configuration, and `nvs config dump` prints every setting that is in
effect. Both commands run no program.

Without arguments, both read `./nvs.toml`. You can name other files. `config check` reads all the
files, with their includes, `[[app]]` blocks and secrets. It prints one line such as
`ok: 2 files, 4 directives set, 1 override, 0 warnings`. The exit status is `1` for a syntax error,
an unknown key or an include that does not exist.

`config dump` prints one key on each line, sorted by name. `--origin` adds the file that set each
key. `--toml` prints the result as one TOML document, which you can compare between two
environments.

**Good to know:** `config check` does not check that a value is a valid amount.
`memory = "12 bananas"` passes it.
