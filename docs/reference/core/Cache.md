---
summary: two caches — `memory` keeps values in the memory of this server process and needs no setup, `shared` is a store on the network such as Redis that every process and machine sees
keywords: cache, memoize, in-memory cache, per-process, redis, ttl, expiry, hit, miss, cross-request state, which cache
---

`Core\Cache` keeps a value after the request that made it ends. It has two caches, and both return a
`Core\Cache\Store` with the same methods: `put`, `get`, `forget`, `putSecret` and `getSecret`.

**`Core\Cache::memory()` keeps values in the memory of this server process.** Every core of one
`nvs serve`, or the one `nvs run`, sees the same values. It needs no configuration and no network,
so it is fast. A restart empties it, and a second server process or machine does not see it. Use it
for ordinary caching: a slow result, a parsed template, a table that many requests read.

**`Core\Cache::shared()` is a store on the network,** such as Redis, at the `[cache.shared] url` the
person who runs the server sets. Every process on every machine sees the same values, and they stay
after a restart. Use it when another server must find the value, or when your program would be
wrong if the value were missing: a session, a lock, a quota.

The memory cache has a size limit, `[cache.memory] max_size`. When it is full, it forgets the value
written longest ago, and `put` does not fail. On both caches any `get` may return `null`, so your
program must be able to build the value again. `put` copies the value into the cache, and `get`
returns a copy.

```nvs
<?nvs
var $cache = Core\Cache::memory();
echo $cache->get('greeting') ?? 'not in the cache', "\n";

// Every request on this server can now read this value.
$cache->put('greeting', 'hello', { ttl: 30s });
echo $cache->get('greeting') ?? 'not in the cache', "\n";

$cache->forget('greeting');
echo $cache->get('greeting') ?? 'forgotten', "\n";
```
```output
not in the cache
hello
forgotten
```
