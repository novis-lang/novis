---
summary: the request-local view of `nvs.toml` — read a directive, move one for this request only, put it back
keywords: ini_get, ini_set, ini_restore, ini_get_all, set_time_limit, nvs.toml, configuration, directive, [limits], memory, cpu_time, hard ceiling
---

`Core\Config` reads and moves the configuration in force for the current request. Every value crosses as a
`string`, spelled as `nvs.toml` would spell it (`"512M"`, `"30s"`); a directive is named with dots
(`log.level`), and a bare limit name — `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` — is the
`[limits]` entry. `set` changes a directive for this request alone and answers `false`, leaving the value in
place, for anything it may not do: a directive only the file may set, an unknown name, a value of the wrong
shape, or a value above the `[limits.hard]` ceiling the operator kept. `restore` puts the file's value back
and `all` lists what is in force. Without an `nvs.toml`, `get` answers `null` for everything.

```toml file=nvs.toml
[limits]
memory = "256M"

[limits.hard]
memory = "512M"
```
```nvs
<?nvs
echo "memory=", Core\Config::get("memory") ?? "unset", "\n";

echo Core\Config::set("memory", "512M") ? "raised to the ceiling" : "refused", "\n";
echo Core\Config::set("memory", "1G") ? "raised past it" : "refused past the ceiling", "\n";
echo "now=", Core\Config::get("limits.memory") ?? "unset", "\n";

Core\Config::restore("memory");
echo "restored=", Core\Config::get("memory") ?? "unset", "\n";

echo Core\Config::set("no.such.directive", "1") ? "set" : "unknown name refused", "\n";
```
```output
memory=256M
raised to the ceiling
refused past the ceiling
now=512M
restored=256M
unknown name refused
```
