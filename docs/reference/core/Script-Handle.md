---
summary: what `spawn script` answers — a handle on a running child script that `await` collects exactly once
keywords: spawn script, await, isolate, child script, subprocess, handle, script.spawn, output capture
---

`Core\Script\Handle` is the value `spawn script 'file.nvs'` answers: the name of a child script already
running in its own isolated memory. **It has no members.** The only thing to do with one is `await` it,
which waits for the child and answers its result — `ok`, `output`, `value` and `error` — and a second
`await` on the same handle throws `LogicError`, because the first one collected the child. A child's own
failure arrives as `ok` being `false`, never as a throw in the parent. Spawning needs the `script.spawn`
capability from `nvs.toml`; the construct, its `with(...)` options and the result's fields are in
[concurrency](#lang-concurrency).

```toml file=nvs.toml
[[app]]
entry = "main.nvs"

[app.capabilities.script]
spawn = true
```
```nvs file=child.nvs
<?nvs
echo "child says hello";
return 6 * 7;
```
```nvs
<?nvs
var $job = spawn script 'child.nvs' with(output: 'capture');
echo "spawned", "\n";

var $done = await $job;
echo "ok=", $done->ok ? "true" : "false", "\n";
echo "output=", $done->output, "\n";
echo "value=", $done->value as int, "\n";

try {
    var $again = await $job;
} catch (LogicError $twice) {
    echo "a handle is awaited once", "\n";
}
```
```output
spawned
ok=true
output=child says hello
value=42
a handle is awaited once
```
