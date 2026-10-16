`[[server.mount]]` carries no `mode` key, and one written there is refused as an unknown key
(`E0601`). A mount **routes** — where requests arrive — and an `[[app]]` block **sets policy** — what
the code that serves them may do:

```toml
[[server.mount]]
path = "/shop"
root = "/srv/www/shop"                     # routing

[[app]]
root = "/srv/www/shop"                     # policy
mode = "production"
```

The two blocks usually name the same directory, and that is the intended shape rather than a
redundancy. Keying policy on the mount would leave `nvs run` with no per-application identity at all,
and would make per-application configuration unreachable until a server existed; keyed on the entry
file, per-application mode works identically for `nvs serve` and `nvs run`, and a mixed-application
host is expressed with `[[app]]` blocks. `[mode] ceiling` still bounds what any `[[app]]` block may
select, so nothing here widens what a mount could previously reach.
