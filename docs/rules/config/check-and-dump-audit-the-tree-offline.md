`rule:config/later-wins-and-every-override-is-recorded` is only safe while it is auditable, so the
reporting is part of the rule rather than tooling around it. `config` is a namespace beside
`service`, and stays out of `nvs check`, which checks source.

```console
$ nvs config check --config /etc/nvs/nvs.toml
ok: 5 files, 47 directives set, 3 overrides, 1 warning

$ nvs config dump --origin
capabilities.process.exec = true      /etc/nvs/conf.d/host.toml
db.main.password          = <secret>  /run/secrets/db_password
limits.memory             = "512M"    /etc/nvs/prod.toml (overrides /etc/nvs/base.toml)
log.format                = "json"    default (production)
log.target                = "stderr"  default

$ nvs config dump --toml > effective.toml     # the written keys, for diffing environments
```

Both are **offline and need no server**, so a tree is validated in CI before it is deployed. `check`
stops at the first refusal, because a file that does not parse has no keys to carry into the rest of
the stream, and its four counts are what CI reads; the directives it counts are the keys the files
wrote. `dump` lists **every key in force**: each key a file wrote, and the default of every key no
file wrote, as `nvs serve` has it (`nvs_config::defaults::unwritten`, server work, the mode the tree
starts in). `dump --origin` is where each override is named, per key, with the file that set it and
the one it overrode; a default's origin is `default`, or `default (production)` /
`default (development)` where the mode chose it (`rule:config/a-mode-is-five-defaults` and the
startup rows). The origin names the file and not the line, because a parsed value carries no span. A
secret renders `<secret>` and names the file its value came from, never the value: the secret file,
or the configuration file for a value written inline. `--toml` writes an inline secret's key with
the value `"<secret>"`, so a diff of two environments shows the key is set, and a file-backed
secret keeps its `_file` key. `--toml` serializes the written table alone: a default written into the document would change the tree it is
read back as (a mode row stops following the mode, a `[limits.hard]` ceiling starts bounding every
`[[app]]` block, a core count or a data-folder path pins it to one machine), so the document read
back lists the same rows.

**A grant entry that allows nothing however it is asked is a refusal, from `check` and at boot
alike**, because both resolve the same tree: a host with a port or a scheme (a grant compares the
bare host), a `*` (only `db.open` and `ldap.open` take a pattern, and theirs is `*.suffix`), `true`
on a grant that has no `true` spelling, a relative SQLite path under `db.open`, a `net.listen` entry
that is not an `address:port`, and a `net.internal` entry that is not an IP address. Each was read as
nothing before, so the deployment that wrote it was denied with no word about why. The message names
the entry and the file, and its help is the line that allows what the entry meant, under its table.

**`check` also reads the programs, and only for the four open blocks.** A tree that writes a key in
`[settings]`, `[secrets]`, `[app.settings]` or `[app.secrets]` has each `[[app]] entry`, each entry
a `[[server.mount]]` reaches and each `--program <file>` compiled, and every key none of them reads
is `W1028`, counted on the summary line with the other warnings
(`rule:config/settings-and-secrets-are-a-programs-own-values`). A tree that writes none of the four
compiles nothing, so the check stays a read of the files. The mount table is expanded as `nvs serve`
expands it, and its refusal is `check`'s.

Both read without the ownership check, and `check --as <account>` runs it for the account that will
serve (`rule:config/the-ownership-check-runs-where-it-can-be-answered`). What a reload actually published is
the record it writes to `Core\Log` (`rule:config/a-reload-names-what-it-could-not-apply`).
