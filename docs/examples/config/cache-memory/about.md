Sets the size limit and the wait time of the memory cache, the one `Core\Cache::memory` uses.

The memory cache is one store in this server process. Every core of the server uses the same store.
`max_size` is how much memory all its values may use together. The default is `32M`. When a new value
does not fit, the value written longest ago is forgotten.

`fill_wait` is for a value that is missing. Several requests may need the same missing secret at the
same time. One request fetches it, and the others wait up to `fill_wait` for that value. The default is
`5s`. A request that is still waiting after that time gets a `TimeoutError`.

**Good to know:** only the person who runs the server may set these two values. A `fill_wait` of zero is
not allowed, and the server does not start with one.
