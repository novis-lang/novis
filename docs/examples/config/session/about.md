Where a session's record lives, how long an untouched one survives, and what the cookie carrying its
identifier is called.

A session must be the same session whichever core or host answers the next request, so there are
exactly two places it may live: the coherent shared store, or a database table. The two weak cache
tiers cannot be written here at all, and a deployment that tries is refused when it starts — a
session kept in a per-process cache is one whose contents depend on who happened to answer, which is
a logged-out user, a lost basket, or somebody else's data, depending on the day.

**In plain words:** a session is not a cache, and this block only offers stores that can prove it.

The cookie's *name* is here and its attributes are not. Every cookie this server writes is already
secure, HTTP-only and same-site, so a second place to spell those would only ever be a way to weaken
them. The whole block is the operator's and is bound when the process starts.

The example prints where sessions live in this deployment, what the cookie is called, and is turned
away moving either.
