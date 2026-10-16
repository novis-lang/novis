`Core\RateLimit::consume` and `::shed` are two jobs with two verbs, and the verb says which one you
are doing. `consume` enforces a policy the application promised somebody — a plan quota, a login
limit — over the shared store, coherently, because a policy only approximately enforced is not
enforced. `shed` drops load to keep a host up, from the serving process's own memory, and an
approximate answer is entirely adequate for that because the goal is "less than the amount that
hurts", not a number. `shed`'s contract states its arithmetic out loud: its count is per server
process, so a limit of 100 on each of four processes admits up to 400 across the deployment, and an
arrival the cache's cap forgets admits a burst again.

`shed` keeps its count in the memory tier (`rule:concurrency/the-memory-tier-is-one-store-per-process`),
scoped by app and configuration generation like any key there and bounded by the same
`[cache.memory] max_size`. The read and the write of one arrival are one step under the shard's write
lock, so every core of a process charges the same count and two cores never both admit the last unit.

They are not two tiers of one operation, and a flag on one member could not carry the difference.

The class exists for the limits only the application can key: five failed logins per *account*, a
quota per tenant plan, ten exports per user per hour. Per-account login throttling is what stops
credential stuffing, which per-IP throttling does not, because the attacker has many addresses and
the victim has one account — and no proxy knows which account a request authenticates as. Left
without a class, an application writes one of two wrong limiters: a counter kept with `get` and `put`
in the memory tier, which two cores race and every process counts on its own, or an insert-and-count
against the database, which puts a write on the hot path of the endpoint it protects. The price of `consume`'s coherence is a network
round trip on that path, the same floor the shared cache tier accepts, and it is why `shed` exists
for the cases that do not need it.

Four things are deliberately absent. **Edge and flood limiting** — no per-IP, per-path or connection
limit as a deployment feature: a proxy owns it earlier and cheaper, and doing it here means paying
for the request in order to reject it. **All configuration** — a limit is application policy, so
there is no `[ratelimit]` block at all. **Any automatic enforcement** — the member returns a decision
and writes no `429`. **Distributed reservation** and multi-key atomic checks, which are considerably
more machinery with no named use case yet.

**Declined:**
- **Only the shared tier.** The two names work only if both exist. With one, the first program that
  needs approximate load shedding uses `consume` for it, and puts a round trip on a path that did not
  need one.
- **One member with a `coherent: true` option.** The default is chosen once and copied, and the wrong
  choice fails silently — the reason `Core\Cache` has two members
  (`rule:core-api/two-cache-tiers`).

- **A middleware or attribute that applies a limit to a route.** It needs a dispatch pipeline Novis
  does not have (`rule:routing/routes-are-compiled-not-registered`), and what to do about a limit is
  application logic.
- **Backing it with `Core\Db`.** It puts a write on the hot path of every protected endpoint, on the
  database that is usually the scarcest resource in the deployment.
