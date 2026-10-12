Whichever step won is **canonicalized to an absolute path once, at boot, and stored**. A reload
re-reads that stored path and never re-resolves the working directory, so a deployment that replaced
its directory under a running process reloads the file the operator can see rather than an unlinked
inode.

The resolved path, and every file the tree reached, are printed at boot:

```console
$ nvs serve
info: configuration ./nvs.toml -> /srv/www/app/nvs.toml
info: resolved from 5 files
info:   /srv/www/app/nvs.toml
info:   /srv/www/app/conf.d/10-limits.toml
info:   /etc/nvs/local.toml
```

The announcement is half of what makes the two implicit steps of
`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` safe — the working directory's
`nvs.toml` and the data folder's: reading the wrong file is visible in one line rather than silent.
When no step finds a file, the shipped-defaults line is printed instead, and the resolved absolute
path is logged in every case.

**Not on disk.** `--config` paths are made absolute (`crates/nvs-config/src/resolve.rs:392`), and a
reload reads the `--config` roots as they were written (`crates/nvs-cli/src/reload.rs:137`). A root
found as `./nvs.toml` or in the data folder is not stored as a canonical path, and boot prints no line
naming the root (`crates/nvs-cli/src/config.rs:537`).
