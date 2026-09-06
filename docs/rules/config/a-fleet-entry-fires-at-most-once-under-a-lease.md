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
`Leases` parameter only `nvs serve` can supply. **Today `nvs serve` supplies none**: the shared tier's
wire is `put` and `get` (`rule:concurrency/a-cached-value-is-copied-across-the-boundary`) and neither
is a set-if-absent, so every `fleet` entry boots, is left **unarmed**, and is named in a boot note.
Firing it on each host's own clock would be the precise failure the scope exists to prevent, so the
safe half is to run none of them and say so.
