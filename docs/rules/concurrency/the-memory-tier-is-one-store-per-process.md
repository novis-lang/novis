`Core\Cache::memory()` hands back one store per **serving process**: coherent across every core of that
process, held in memory only, capped by `[cache.memory] max_size`, and gone when the process ends. It is
the tier for anything a process would otherwise compute again on every request — a parsed configuration,
a compiled template, a token fetched from a provider — and it answers the same store class `shared()`
does.

It is a **cache and not a store**, with every sentence `rule:concurrency/cross-request-state-is-explicit`
writes about it holding: an entry may be absent at any time — for the cap, for its lifetime, or because
this is a different process than the one that wrote it — and a program that would be incorrect on a miss
is using the wrong tier (`rule:concurrency/the-memory-tier-cannot-hold-what-must-be-coherent`). A value is
copied in and copied out by the same graph copy both tiers use
(`rule:concurrency/a-cached-value-is-copied-across-the-boundary`).

This is the mutable state the cores share, so the map is sharded behind read/write locks and `get` takes
a read lock and never a write, which is what keeps a lookup off a `&mut` on the request path. The shard
count is a fixed constant rather than a directive or a function of `[server] workers`: the map is created
before the workers exist under `nvs serve`, and there is one worker under `nvs run`. The constant is 64:
a mask over the one hash the key already needs, so sharding costs no second hash, and 64 covers the core
counts a serving box plausibly has. Contention here is write-side only, and sharding is invisible to a
program, so the number can move without anything in this contract changing.

**Its memory is not attributable to a request.** It is the deliberate, bounded exception to the rule
that every byte belongs to a request in flight: held by the process, capped by an `nvs.toml` directive,
and kept off every request's reading by the balance
`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance` describes. **The cap evicts rather
than failing a write.** A `put` that crosses it forgets the entry written longest ago and succeeds, so a
program never sees a store fail because it was full, which is the whole point of a store whose contract
already says any entry may be absent. Each shard evicts against an even share of the cap, so the
largest entry the tier holds is that share. An expired entry is not swept by a timer: a sweep is a timer
and a write pass over a shard, and what it would buy, memory returned slightly sooner, the cap already
buys.

`[cache.memory] max_size` ships at `32M`, `System`-class and applied on reload, and
`[cache.memory] fill_wait` belongs to the same block (`rule:concurrency/a-secret-fill-runs-once-per-process`).

An entry's real key carries the `[[app]]` and the configuration generation that was live when it was
written, so two apps on one server never read each other's entries and a reload never serves an entry
written under the configuration it replaced.

No capability gates it: a capability is checked at the door to an *effect*, and this tier has no door.
Nothing leaves the process, no name is resolved and no file is opened. What is bounded instead is
footprint, and the cap is the instrument for that.

- **Memory:** at most `[cache.memory] max_size` bytes, once per process, on the detached balance:
  O(working set), never O(requests served). An entry's lifetime is governed by its TTL and the cap,
  never by how much traffic has passed through. On top of it, 64 locks and 64 empty maps, allocated
  once at process start.

**Declined:**
- **A `[cache.memory] shards` key, or a shard count derived from `[server] workers`.** The map exists
  before the workers do, a derived count would size the CLI's map at one shard, and 64 empty shards are
  below the granularity at which footprint is worth a directive.
- **A sweeper thread for expired entries.** The cap already evicts, and a sweep is a timer and a write
  pass over a shard for memory returned slightly sooner.
- **A capability key for this tier.** It has no door to an effect, so there is nothing for a grant to
  check (`rule:core-api/two-cache-tiers`).
- **Failing a `put` once the cap is reached.** It turns a full cache into an error every caller has to
  handle, for a store whose every entry may already be absent.
