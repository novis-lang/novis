Stores values in the memory of this server process, where every core of the server finds them.

You write a value with `put` and read it back with `get`. A key that nothing wrote returns `null`, and
any value may be gone at any time. The store has a size limit, 32 MB by default. When a new value does
not fit, the value written longest ago is forgotten.

This is the tier for ordinary caching. A later request finds what an earlier one wrote, whichever core
it runs on, which `Core\Cache::local` cannot promise. The store is memory only, so it starts empty when
the server starts, and nothing in it survives a restart.

**Good to know:** a second server machine has a store of its own. If every machine must see the same
value, use `Core\Cache::shared`.

**The examples below** show a write and a read first, then how this store and the per-core one stay
apart, then a slow answer fetched once and then used by every request.
