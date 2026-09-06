Both members implement the **generic cell rate algorithm** — a leaky bucket expressed as one stored
timestamp, the theoretical arrival time of the next permitted request. `limit` per `per` sets the
drain rate, `burst` (defaulting to `limit`) sets how much may arrive at once, and `cost` weights one
call so an expensive endpoint may consume five units of the same quota.

One duration of arithmetic per call, and **one stored timestamp per key** rather than a window of
them. In the shared store that is the difference between O(1) and O(requests in window) memory per
key, which is what makes limiting per user affordable at a million users. `retryAfter` is computed
rather than estimated, where a sliding-window counter can only answer "sometime in the next window" —
the answer that makes clients retry in a burst at the window edge. Both tiers run the identical
algorithm, so moving a call between them changes the *guarantee* and not the shape of the answer.

The `Decision` is readonly `allowed`, `limit`, `remaining` and `retryAfter`, the last being `?Duration`
and `null` exactly when `allowed` is true — absence is `?T`, not a sentinel zero. Both members are
qualifier-neutral. The key **accepts `tainted`**, since a tenant or account id is user-derived by
nature and the store's protocol is length-prefixed, so a key cannot reshape a command; it
**refuses `secret`**, because using a signing key as a rate-limit key writes it into a store with a
TTL. The in-process tier is an existing crate; the shared tier is a small script of our own, an
algorithm over our own state.
