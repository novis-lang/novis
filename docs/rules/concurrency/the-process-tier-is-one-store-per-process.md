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
created before the workers exist under `nvs serve`, and there is one worker under `nvs run`. Eviction is by
write age, as it is in the local tier, and a full tier forgets rather than failing a `put`.

An entry's real key carries the `[[app]]` and the configuration generation that was live when it was
written, so two apps on one server never read each other's entries and a reload never serves an entry
written under the configuration it replaced.

No capability gates it, for the reason the local tier needs none: a capability is checked at the door to an
*effect*, and this tier has no door. What is bounded instead is footprint —
`[cache.process] max_size`, `System`-class and applied on reload — and every byte of it moves the detached
balance under a bracket the store itself holds
(`rule:concurrency/a-cross-request-stores-bytes-are-its-own-balance`). What it spends is O(working set)
once per process, where the local tier is O(cores × working set), and never O(requests served).
