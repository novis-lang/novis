`rule:config/later-wins-and-every-override-is-recorded` is only safe while it is auditable, so the
reporting is part of the rule rather than tooling around it. `config` is a namespace beside
`service`, and stays out of `nvs check`, which checks source.

```console
$ nvs config check --config /etc/nvs/nvs.toml
ok: 5 files, 47 directives set, 3 overrides, 1 warning

$ nvs config dump --origin
limits.memory             = "512M"    prod.toml:4   (overrides base.toml:2)
capabilities.process.exec = true      conf.d/host.toml:3
db.main.password          = <secret>  /run/secrets/db_password (conf.d/20-db.toml:5)

$ nvs config dump --toml > effective.toml     # one canonical file, for diffing environments
```

Both are **offline and need no server**, so a tree is validated in CI before it is deployed. `check`
stops at the first refusal, because a file that does not parse has no keys to carry into the rest of
the stream, and its four counts are what CI reads; `dump --origin` is where each override is named,
per key, with the file and line that set it and the one it overrode. A secret renders `<secret>` and
names the file it came from, never the value, and `--toml` serializes the merged table whole.

**A grant entry that allows nothing however it is asked is a refusal, from `check` and at boot
alike**, because both resolve the same tree: a host with a port or a scheme (a grant compares the
bare host), a `*` (only `db.open` and `ldap.open` take a pattern, and theirs is `*.suffix`), `true`
on a grant that has no `true` spelling, a relative SQLite path under `db.open`, a `net.listen` entry
that is not an `address:port`, and a `net.internal` entry that is not an IP address. Each was read as
nothing before, so the deployment that wrote it was denied with no word about why. The message names
the entry and the file, and its help is the line that allows what the entry meant, under its table.

Both read without the ownership check, and `check --as <account>` runs it for the account that will
serve (`rule:config/the-ownership-check-runs-where-it-can-be-answered`). What a reload actually published is
the record it writes to `Core\Log` (`rule:config/a-reload-names-what-it-could-not-apply`).
