Whichever step won is **canonicalized to an absolute path once, at boot, and stored**. A reload
re-reads that stored path and never re-resolves the working directory or the data folder, so a
deployment that replaced its directory under a running process reloads the file the operator can see
rather than an unlinked inode, and a `nvs.toml` that appears in the working directory later is not
read until the next boot. A boot on the shipped defaults reloads the shipped defaults.

`nvs serve` prints the root as it was found, the path it was read from, and every file the tree
reached, on standard error before it binds:

```console
$ nvs serve
info: configuration ./nvs.toml -> /srv/www/app/nvs.toml
info: resolved from 3 files
info:   /srv/www/app/nvs.toml
info:   /srv/www/app/conf.d/10-limits.toml
info:   /etc/nvs/local.toml
```

The announcement is half of what makes the two implicit steps of
`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` safe — the working directory's
`nvs.toml` and the data folder's: reading the wrong file is visible in one line rather than silent.
When no step finds a file, `info: configuration: no file was found, so the shipped defaults apply`
is printed instead. When step 4's write into the data folder declines, its reason is one more `info:`
line ahead of that one. Other commands print none of these, so `nvs run`'s standard error stays the
program's own.

The stored roots are `crates/nvs-cli/src/config.rs`'s `Root`, and a reload resolves them through
`stored_set`.
