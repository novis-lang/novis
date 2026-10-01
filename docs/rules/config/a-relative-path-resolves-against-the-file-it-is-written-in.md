Every path-valued directive — `opcache.file_cache_dir`, `capabilities.script.spawn`, `debug.trace`,
`[[extension]] path`, `[[server.mount]] root`, `[db.<name>] path`, `[storage.<name>] root`, `password_file`, an `[[app]]`
block's `root` or `entry`, `[[schedule]] script`, `[log] handler`, and `[[include]]`'s own `path` and `dir` — resolves relative to the
directory of the file the value appears in. A path given on the **command line** resolves against the
working directory, because that is what a shell argument means.

```toml
# /etc/nvs/nvs.toml
[[include]]
path = "conf.d/db.toml"            # -> /etc/nvs/conf.d/db.toml

# /etc/nvs/conf.d/db.toml
[db.main]
path = "data/app.sqlite"           # -> /etc/nvs/conf.d/data/app.sqlite
```

One rule shared with `[[include]]`, and the only one under which a configuration directory survives
being copied or relocated whole. The resolved absolute path is what the boot log and `nvs config dump`
print, so the rule never has to be applied in a reader's head — and a block in an included file names
a database beside *that* file, not beside the running program. A scheduled script and a log handler
are resolved while the configuration is read, so every fire and every escalation runs the same file
whatever folder the server was started in.
