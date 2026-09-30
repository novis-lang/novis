`nvs config check` and `nvs config dump` show what a configuration contains, and they run no
program.

`nvs config check` reads the file and every file that it includes. It prints one line with the
number of files, settings, overrides and warnings, such as
`ok: 1 file, 4 directives set, 0 overrides, 0 warnings`. The exit status is not `0` when the
configuration has an error.

`nvs config dump` prints every key that is in effect, one key on each line. `--origin` adds the
file that set each key and the value that it replaced. `--toml` prints the result as one TOML
document.

**Good to know:** the dump prints the full name of each key. A key in the first `[[app]]` block
starts with `app.0.`, for example `app.0.root`.
