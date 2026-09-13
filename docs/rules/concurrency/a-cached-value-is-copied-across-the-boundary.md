`put` copies a value out of the request heap into the cache; `get` copies one back in. The operation
is the recursive graph copy the language already defines and already shares with the isolate
boundary — not a third mechanism.

This is forced rather than chosen. A request's heap is dropped wholesale when the request ends, so a
value the cache holds cannot live there; and it cannot be handed back by reference either, or that
wholesale drop would free something the cache still holds. Reference-sharing would make a heap's
lifetime depend on what a request happened to read, which is exactly the property the wholesale drop
exists to guarantee.

A cached value is therefore subject to every restriction any crossing value is: a generator does not
go in, and neither does a `secret`. Both are refused at `put` rather than silently degraded, and for
a secret the door is elsewhere: `putSecret` seals it and puts the **ciphertext** across this boundary
as `bytes`, so what the copy sees is never a `secret`
(`rule:concurrency/a-secret-is-cached-only-sealed`).

The copy is a real per-`get` cost, paid to keep the request model intact. An implementation may
later share immutable scalars within a core by refcount, since a core is single-threaded — an
optimisation, and it must not be observable.
