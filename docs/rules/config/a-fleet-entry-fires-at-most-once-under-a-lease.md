A `scope = "fleet"` entry fires once per interval across the deployment, guarded by a **lease** in the
shared store keyed on the entry's `name` plus the fire's *scheduled* instant — not the instant it was
noticed, so two hosts whose clocks differ by a second still ask for the same key. A host that wins the
lease runs the script; a host that does not, does not. The lease carries a TTL and is renewed while
the run is in flight, so a host that dies mid-run releases it by expiry rather than blocking the next
interval forever.

**`"fleet"` is at-most-once per interval, not exactly-once.** A network partition can leave an
interval unrun; a lease expiring under a run that is alive but unreachable can produce a second run.
Exactly-once across machines needs a transaction the work itself participates in, which is the
application's job and not a scheduler's: a job that must not run twice makes its own effect
idempotent.

The ticker holds no store: it asks one question — take this key for this long, yes or no — through a
`Leases` parameter only `nvs serve` can supply, because the crate the ticker lives in names no
standard library. That binary supplies one whenever the tree names a `[cache.shared]` store it can
reach, over a connection of its own to that tier and a set-if-absent no program is given
(`rule:concurrency/cross-request-state-is-explicit`). The connection is opened at boot, and again
when a reload moves `[cache.shared]` or changes a roster that needs a lease and has none. The next
fire takes its key in the new store, and a fire already running renews in the store it took its key
from.

A tree with no shared store, and one whose store will not answer, leave every `fleet` entry
**unarmed** and named in a note. Firing it on each host's own clock would be the precise failure
the scope exists to prevent, so the safe half is to run none of them and say so.
