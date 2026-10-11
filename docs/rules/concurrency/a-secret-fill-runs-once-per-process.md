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
rather than a duration because the tier does not know what a lifetime means to its caller. A fifth leaves
four fifths of every lifetime untouched and gives a fetch room to fail once and be retried by the next
caller before anybody waits. Refresh-ahead is `getSecret`-with-a-`fill` and nothing else: a plain `get`
past its lifetime is simply absent.

**`[cache.process] fill_wait` ships at `5s`,** `System`-class and applied on reload. The filler may spend
its whole `[http.client] deadline`, which ships at `30s`, but a waiter is a request held on the latency
path, and a slow provider that held every in-flight request on the box for half a minute is the outage
this tier exists to prevent. Five seconds is generous for a token endpoint and short enough that a caller
gets a `TimeoutError` it can answer. So a `fill` slower than the wait succeeds for the filler while its
waiters throw, and the callers after it find the entry; an operator whose provider is slower raises the
key. It is `System`-class because how long one core's request may block on another's fetch is not a
bargain one request gets to strike for everyone, and a caller who wants a different wait writes `wait` at
the call site, where review sees it. A zero wait and `false` are refused at boot under `E0642`: zero makes
every concurrent caller but one throw, the stampede's failure with the fetches removed, and there is no
spelling for an unbounded wait (`rule:http-server/no-spelling-for-an-unbounded-wait`).

It is once per **process**, not once per fleet. On the shared tier every other process runs its own `fill`,
and a fleet-wide single fetch is a lease over that store — an owner, a renewal and a recovery path — rather
than anything this key spells. This is also the one step in `Core\Cache` that observes an entry and writes
it, and it is not a coordination primitive
(`rule:concurrency/put-and-get-are-the-whole-boundary`): a program cannot take the fill, cannot see who
holds it, and cannot make one key's fill wait on another's. What it buys is that the fetches a fleet makes
against a provider are bounded by its process count rather than its request rate.

- **Memory:** the fill table holds one slot per key being filled at that moment: O(concurrent fills),
  not O(keys) and not O(requests).

**Declined:**
- **Caching a `fill` failure**, so an outage is not re-attempted by every caller. It puts one request's
  exception into another's path, and the cure for a hammered provider is the caller's own retry policy,
  which is already jittered and bounded.
- **A fleet-wide single fill.** It is a lease, not a cache entry: an owner, a renewal, recovery from an
  owner that died mid-fetch, and a story for a network partition, none of which belongs behind a `fill`
  key.
- **A refresh-ahead window of a tenth or a half.** A tenth of a short lifetime is a window a slow fetch
  does not fit inside, which puts callers back into the wait refresh-ahead exists to remove; a half
  doubles the fetch rate against the provider for values that are still good.
