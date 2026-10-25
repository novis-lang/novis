---
summary: the request-local view of `nvs.toml` — read a directive, move one for this request only, put it back
keywords: nvs.toml, configuration, setting, read a setting, change a setting at run time, directive, [limits], memory, cpu_time, time limit, hard ceiling
---

`Core\Config` reads and moves the configuration in force for the current request. Every value crosses as a
`string`, spelled as `nvs.toml` would spell it (`"512M"`, `"30s"`); a directive is named with dots
(`log.level`), and a bare limit name — `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` — is the
`[limits]` entry. `set` changes a directive for this request alone and answers `false`, leaving the value in
place, for anything it may not do: a directive only the file may set, an unknown name, a value of the wrong
shape, or a value above the `[limits.hard]` ceiling the operator kept. When `[limits.hard]` does not
set a limit, its ceiling is the value the request started with, so a program cannot raise it. A program you
start from the command line is the one exception: with no `[limits.hard] memory`, it may raise `memory` to any value. `restore` puts the file's value back,
or the built-in default when the file does not write the key. `get` and `all` give the built-in default of every key the file
does not write, such as `128M` for `memory`. A key with no default, such as `wall_time`, is `null`.

```toml file=nvs.toml
[limits]
memory = "256M"

[limits.hard]
memory = "512M"
```
```nvs
<?nvs
echo 'memory=', Core\Config::get('memory') ?? 'unset', "\n";

echo Core\Config::set('memory', '512M') ? 'raised to the ceiling' : 'refused', "\n";
echo Core\Config::set('memory', '1G') ? 'raised past it' : 'refused past the ceiling', "\n";
echo 'now=', Core\Config::get('limits.memory') ?? 'unset', "\n";

Core\Config::restore('memory');
echo 'restored=', Core\Config::get('memory') ?? 'unset', "\n";

echo Core\Config::set('no.such.directive', '1') ? 'set' : 'unknown name refused', "\n";
```
```output
memory=256M
raised to the ceiling
refused past the ceiling
now=512M
restored=256M
unknown name refused
```
