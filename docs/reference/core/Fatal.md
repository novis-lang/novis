---
summary: the one hook that runs after a resource limit has stopped the request — what `register_shutdown_function` was for on a fatal
keywords: register_shutdown_function, memory limit, resource limit, FATAL, onLimit, cpu_time, wall_time, max_output, out of memory, shutdown handler
---

`Core\Fatal::onLimit` registers the callable the request runs when a resource limit — memory, CPU time,
output, wall time, script depth or call-stack depth — stops it. A limit breach is a `FATAL`, which no
`catch` sees; the handler is the only code that observes one. It runs once, out of a slice of the request's
budget reserved for it, and never a second time: a handler that throws or exhausts that slice is abandoned
where it stands. Registering is request-local, a later call replaces the earlier handler, and registering
runs nothing. The handler receives one array whose `limit` key names the directive that fired, spelled as
`nvs.toml` spells it (`memory`, `cpu_time`); it may declare no parameter at all.

```nvs
<?nvs
Core\Fatal::onLimit(fn (array<string> $report): void => {
    echo "stopped by the ", $report["limit"], " limit\n";
});
echo "handler registered\n";

Core\Fatal::onLimit(fn (): void => {
    echo "a replacement that takes no report\n";
});
echo "handler replaced\n";

var $work = Core\Arr::range(1, 1000);
echo "work done: ", Core\Arr::count($work), " items, no limit reached\n";
```
```output
handler registered
handler replaced
work done: 1000 items, no limit reached
```
