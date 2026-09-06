`Core\RateLimit::consume` and `::shed` are two jobs with two verbs, and the verb says which one you
are doing. `consume` enforces a policy the application promised somebody — a plan quota, a login
limit — over the shared store, coherently, because a policy only approximately enforced is not
enforced. `shed` drops load to keep a host up, per core, and an approximate answer is entirely
adequate for that because the goal is "less than the amount that hurts", not a number. `shed`'s
contract states its arithmetic out loud: its count is per core, so a limit of 100 across eight cores
admits up to 800.

They are not two tiers of one operation, and a flag on one member could not carry the difference.

Four things are deliberately absent. **Edge and flood limiting** — no per-IP, per-path or connection
limit as a deployment feature: a proxy owns it earlier and cheaper, and doing it here means paying
for the request in order to reject it. **All configuration** — a limit is application policy, so
there is no `[ratelimit]` block at all. **Any automatic enforcement** — the member returns a decision
and writes no `429`. **Distributed reservation** and multi-key atomic checks, which are considerably
more machinery with no named use case yet.
