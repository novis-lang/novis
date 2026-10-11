`Core\Cache::process()` hands back one store per **serving process** — coherent across every core of that
process, held in memory only, and gone when the process ends. It is the third tier, answering the same
store class `local()` and `shared()` do, and it is a general tier: what belongs in it is anything a whole
machine would otherwise hold once per core.

It is a **cache and not a store**, with every sentence
`rule:concurrency/cross-request-state-is-explicit` writes about the local tier holding here: an entry may
be absent at any time — for the cap, for its lifetime, or because this is a different process than the one
that wrote it — and a program that would be incorrect on a miss is using the wrong tier
(`rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent`). A value is copied in and copied out
by the same graph copy every tier uses (`rule:concurrency/a-cached-value-is-copied-across-the-boundary`).

This is the first mutable state the cores share, so the map is sharded behind read/write locks and `get`
takes a read lock and never a write, which is what keeps a lookup off a `&mut` on the request path. The
shard count is a fixed constant rather than a directive or a function of `[server] workers`: the map is
created before the workers exist under `nvs serve`, and there is one worker under `nvs run`. The constant
is 64: a mask over the one hash the key already needs, so sharding costs no second hash, and 64 covers the
core counts a serving box plausibly has. Contention here is write-side only, and sharding is invisible to
a program, so the number can move without anything in this contract changing. Eviction is by write age, as
it is in the local tier, and a full tier forgets the key written longest ago rather than failing a `put`.

`[cache.process] max_size` ships at `32M`, the figure `[cache.local] max_size` ships. The same number
means less memory here: this tier is held once per process where the local tier is held once per core, so a
working set moved from `local()` to `process()` at stock configuration lowers the machine's ceiling from
`32M` times `[server] workers` to `32M`. An expired entry is not swept by a timer: a sweep is a timer and
a write pass over a shard, and what it would buy, memory returned slightly sooner, the cap already buys.

An entry's real key carries the `[[app]]` and the configuration generation that was live when it was
written, so two apps on one server never read each other's entries and a reload never serves an entry
written under the configuration it replaced.

No capability gates it, for the reason the local tier needs none: a capability is checked at the door to an
*effect*, and this tier has no door. What is bounded instead is footprint —
`[cache.process] max_size`, `System`-class and applied on reload — and every byte of it moves the detached
balance under a bracket the store itself holds
(`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`).

- **Memory:** at most `[cache.process] max_size` bytes, once per process, on the detached balance:
  O(working set), where the local tier is O(cores × working set), and never O(requests served). On top
  of it, 64 locks and 64 empty maps, allocated once at process start.

**Declined:**
- **A `[cache.process] shards` key, or a shard count derived from `[server] workers`.** The map exists
  before the workers do, a derived count would size the CLI's map at one shard, and 64 empty shards are
  below the granularity at which footprint is worth a directive.
- **A sweeper thread for expired entries.** The cap already evicts, and a sweep is a timer and a write
  pass over a shard for memory returned slightly sooner.
- **A capability key for this tier.** It has no door to an effect, so there is nothing for a grant to
  check (`rule:core-api/two-cache-tiers`).
