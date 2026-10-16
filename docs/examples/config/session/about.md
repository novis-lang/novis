Where a session's record lives, how long an untouched one survives, and what the cookie carrying its
identifier is called.

A session must be the same session whichever server process or host answers the next request. So a
session can be kept in only two places: the shared store, or a database table. `Core\Cache::memory()`
is kept inside one server process, so `backend = "memory"` is not allowed. A server with that line
in `nvs.toml` does not start. A session kept in one process is lost when another process answers or
the server restarts. The user is logged out, or their basket is empty.

**In plain words:** a session is not a cache, and this block only offers stores that can prove it.

The cookie's *name* is here and its attributes are not. Every cookie this server writes is already
secure, HTTP-only and same-site, so a second place to spell those would only ever be a way to weaken
them. The whole block is the operator's and is bound when the process starts.

The example prints where sessions live in this deployment, what the cookie is called, and is turned
away moving either.
