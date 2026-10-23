Whichever step won is **stored once, at boot, as the absolute path it was given, never its
canonical path**. A reload resolves that stored path again, through its links, and never re-resolves
the working directory or the data folder. So a swapped `current` link or a changed Kubernetes
ConfigMap is read by the next reload without a restart, a deployment that replaced its directory
under a running process reloads the file the operator can see rather than an unlinked inode, and a
`nvs.toml` that appears in the working directory later is not read until the next boot. A boot on
the shipped defaults reloads the shipped defaults. The configuration check stats each stored root
as given, beside the files the tree reached, and a stamp carries the file's identity, so a link that
points at a new file with the same time and size is still noticed. Because the link decides the
file, the directory that holds each link on the stored path is ownership-checked like the file's own
(`rule:config/ownership-is-the-trust-boundary`), at boot and on every reload.

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
