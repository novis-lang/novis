`nvs config check` checks a configuration, and `nvs config dump` prints every setting that is in
effect. Both commands run no program.

Without arguments, both read `./nvs.toml`. If that file does not exist, they read `nvs.toml` in the
data folder. `config check` reads all the files, with their includes, `[[app]]` blocks and secrets.
It prints one line such as `ok: 2 files, 4 directives set, 1 override, 0 warnings`. The exit status
is `1` for a syntax error, an unknown key or a missing include. It is also `1` for a `[limits]` value
that is not valid, or that is above its `[limits.hard]` ceiling.

`config check --as <account>` also checks the owner of each file and folder for that account. The
first file that a server with this account would not accept stops the check with `E0607`. On
Windows, `NT SERVICE\<service>` works before the service exists. The check sees only the files on
this machine.

`config dump` prints one key on each line, sorted by name. It also prints the default of every key
that no file sets. `--origin` adds the file that set each key, or `default`. `--toml` prints only
the keys that files set, as a TOML document.
