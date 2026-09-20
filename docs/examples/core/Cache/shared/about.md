Stores values in a store outside this server, where every core and every machine finds the same value.

The person who runs the server says which store that is, and grants the capability `cache.shared`. A
program cannot choose the store for itself. You then write a value with `put` and read it back with
`get`, which are the same two methods the two caches in memory have.

This is the tier that answers the same way everywhere. A store the server cannot reach throws an error
instead of answering `null`, so a missing value and a broken store never look the same to your program.
That is the difference from `Core\Cache::local` and `Core\Cache::process`, where a `null` can mean
either one.

**Good to know:** every read and every write here goes over the network, so this tier is slower than
the two caches in memory. Use it for what has to be right, not for what is only nice to have.

**The examples below** show the settings a shared store needs first, then what an unreachable store
does, then how a request falls back to a cache in memory.
