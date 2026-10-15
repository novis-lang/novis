Stores values in a store on the network, where every server process and every machine sees the same values.

The person who runs the server sets up the store in `nvs.toml` and grants the capability `cache.shared`. A
program cannot choose the store for itself. You write a value with `put` and read it back with `get`. These
are the same methods `Core\Cache::memory` has.

Use this cache when another server must find the value, such as a login session. Use it also when your
program would be wrong if the value were missing. A key that nothing wrote returns `null`. If the server
cannot reach the store, the call throws an error. So your program can tell the two cases apart.

**Good to know:** every read and every write goes over the network, so this cache is slower than
`Core\Cache::memory`. For a value that this server can build again, use `Core\Cache::memory`.

**The examples below** show the settings a shared store needs first, then what happens when the server
cannot reach the store, then a request that uses the memory cache in that case.
