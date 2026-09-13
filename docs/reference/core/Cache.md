---
summary: three cache tiers with three promises — `local` is one core's own memory and the next request may not see it, `process` is one map for the whole `nvs serve` (or `nvs run`) process, `shared` is an external store such as Redis that every process and machine sees
keywords: apcu_store, apcu_fetch, apcu_delete, apcu_exists, shmop, sysvshm, cache, memoize, per-core, per-process, redis, ttl, coherence, hit, miss, cross-request state
---

`Core\Cache` is where a value outlives the request that made it, in three tiers that hand back the
same `Core\Cache\Store` (`put`, `get`, `forget`, `putSecret`, `getSecret`) under three different
promises. **`local()` is a map inside one core's own memory**: a serving process runs one core per
CPU, each new connection goes to whichever core is free, and nothing sends a client back to the core
that served it before — so a value one request writes **may or may not** be there when the next
request asks, because that request is usually on a different core with an empty map of its own. It
is the fastest tier, with nothing locked and nothing shared, and it fits only a value every core can
cheaply rebuild for itself: a hot lookup table, a parsed template, a per-core counter like the one
`Core\RateLimit::shed` keeps. **`process()` is one map for the whole process** — every core of one
`nvs serve`, or the single core of one `nvs run` — so a follow-up request on the same machine finds
what an earlier one wrote, at the cost of a lock on each access; it is the tier for ordinary
application caching. **`shared()` is an external store**, Redis at the `[cache.shared] url` an
operator configured, the only tier where a write is seen by every process on every machine and
survives a restart, and the only one for anything two requests must agree on — sessions, locks,
quotas, idempotency keys. On every
tier a `get` may answer `null`, a program that would be *wrong* on `null` is on the wrong tier, a
value is copied in on `put` and out on `get` rather than shared live, and a full tier forgets old
entries instead of failing a `put`.

```nvs
<?nvs
var $local = Core\Cache::local();
var $greeting = $local->get('greeting');
echo $greeting ?? 'miss on this core', "\n";

// The one thing `local` promises: your own write, read back in the same request.
$local->put('greeting', 'hello', {ttl: 30s});
echo $local->get('greeting') ?? 'miss on this core', "\n";

// Every core of the process reads and writes this one map.
var $process = Core\Cache::process();
$process->put('rendered', '<p>hello</p>');
echo $process->get('rendered') ?? 'miss in this process', "\n";

$process->forget('rendered');
echo $process->get('rendered') ?? 'forgotten', "\n";
```
```output
miss on this core
hello
<p>hello</p>
forgotten
```
