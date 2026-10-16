Cross-request state is reached through **a member per tier, each with its own contract**, not one member
with a flag. `Core\Cache::memory()` is one store per serving process, coherent across every core of it,
in memory only and gone when the process ends; any entry may be absent at any time for any reason
(`rule:concurrency/the-memory-tier-is-one-store-per-process`). `Core\Cache::shared()` is a real store
over the network, coherent across processes and machines, and gated by the capability that names the
store an operator configured. No member takes an argument, so there is no flag to have written.

A program that would be incorrect if a read returned nothing is using the wrong tier, and the whole value of
a member per tier is that the choice is made in the source and visible in review. One name covering
different guarantees invites using the weaker one by accident — the same reason a generic sanitizer is
refused.

The memory tier needs no capability, because a capability is checked at the door to an *effect* and it
has no door: nothing leaves the process, no name is resolved and no file is opened. What is left to bound
is footprint, and a configured size cap is the instrument for that — `[cache.memory] max_size` per
process; a boolean grant is not one, and adding it would price a tier as an authority question every
deployment then has to answer.

The memory tier is fast because it never leaves the process: no wire format, no socket and no syscall,
only a graph copy and a read lock on one shard. That is what the parsed-configuration and
compiled-template cases, most of what a cache is used for, need, and none of them needs coherence beyond
one process.

**Declined:**
- **One member with a `coherent: true` flag, or a `store(tier: Cache\Tier)` argument.** The default would
  be chosen once and copied from then on, and choosing wrong fails silently; once a variable can carry the
  tier, the choice stops being visible in review. A name per tier costs a few characters and makes the
  guarantee visible.
- **A secrets-only store such as `Core\Secrets`.** Ordinary values need the same in-process store, so a
  secrets-only one would sit beside `memory()` as a second shape for one mechanism. A secret reaches
  both tiers through a sealed pair on the store class instead
  (`rule:concurrency/a-secret-is-cached-only-sealed`).
- **A token kept in a process-wide static of the caller's own.** It is the ambient cross-request state
  `rule:security/no-cross-request-state` closes, which is why `Core\Cache` exists.
