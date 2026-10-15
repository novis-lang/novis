Stores values in the memory of this server process, so a later request can use what an earlier one built.

You write a value with `put` and read it back with `get`. Every core of the server sees the same values.
The store needs no configuration and no network, so it is fast. It has a size limit, 32 MB by default.
When the store is full, the value written longest ago is forgotten.

Any value may be gone at any time, and a key that nothing wrote returns `null`. So write here only what
your program can build again. This is the cache to use for ordinary caching.

**Good to know:** a restart empties the store. A second server process, or a second machine, has a store
of its own and does not see these values. If another server must find the value, or your program would be
wrong without it, use `Core\Cache::shared`.

**The examples below** show a write and a read first, then a value with a lifetime, then a slow result
that one request fetches and later requests find ready.
