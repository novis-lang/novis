On a miss, `getSecret`'s `fill` runs in **exactly one caller per process**, in that caller's own request and
under its own capabilities, while every other caller on every core waits for it — each for at most its
`wait`, which inherits `[cache.process] fill_wait`, and past which it throws `TimeoutError`. `fill` answers
a `Core\Cache\SecretEntry`, which carries the value and the lifetime the fetch learned, a token endpoint
being the authority on its own.

**A failure is not shared.** When `fill` throws, the waiters are released with nothing and the next caller
to ask runs `fill` itself: an exception crossing from one request into another would be exactly the
cross-request state `rule:security/no-cross-request-state` closes, and a cached failure turns one bad minute
at a provider into an outage every later request inherits. A request that ends while holding the fill
releases it, so a cancelled fetch never wedges a key, and a `fill` that asks for its own key is a
`LogicError` rather than a wait on itself.

**An entry inside the last fifth of its lifetime is still answered to every caller** while exactly one of
them runs `fill` to replace it, so the expiry of a hot key costs nobody a wait. The window is a fraction
rather than a duration because the tier does not know what a lifetime means to its caller. Refresh-ahead is
`getSecret`-with-a-`fill` and nothing else: a plain `get` past its lifetime is simply absent.

It is once per **process**, not once per fleet. On the shared tier every other process runs its own `fill`,
and a fleet-wide single fetch is a lease over that store — an owner, a renewal and a recovery path — rather
than anything this key spells. This is also the one step in `Core\Cache` that observes an entry and writes
it, and it is not a coordination primitive
(`rule:concurrency/put-and-get-are-the-whole-boundary`): a program cannot take the fill, cannot see who
holds it, and cannot make one key's fill wait on another's.
