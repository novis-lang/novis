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

The announcement is half of what makes the working-directory step of
`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` safe: reading the wrong file
is visible in one line rather than silent. Missing `./nvs.toml` prints the shipped-defaults line
instead, and the resolved absolute path is logged in both cases.
