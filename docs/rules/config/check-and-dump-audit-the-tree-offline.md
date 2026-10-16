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
names the file it came from, never the value, and `--toml` serializes the merged table whole. Both
read without the ownership check
(`rule:config/the-ownership-check-runs-where-it-can-be-answered`). What a reload actually published is
the record it writes to `Core\Log` (`rule:config/a-reload-names-what-it-could-not-apply`).
