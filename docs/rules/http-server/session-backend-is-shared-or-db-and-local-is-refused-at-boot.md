```toml
[session]
backend = "shared"      # or "db"; there is no third value and no default that reaches a store
ttl     = "2h"          # how long an untouched record survives; the store enforces it
cookie  = "nvsid"       # the name the identifier rides under
```

`shared` is the coherent tier reached at `[cache.shared] url`
(`rule:config/cache-shared-is-the-grant-over-the-configured-store`); `db` is a table in a
`[db.<name>]`. `backend = "local"` is `E0626`, and its note names
`rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` and the file the key was
written in. That is what "enforced rather than documented" means: the value is refused where it
is written, not where it is used, so a deployment cannot be running on a per-core session store
while believing otherwise. A generic unknown-value refusal would not do it — an operator who wrote
`local` because APCu was where their sessions lived needs the sentence explaining why the fast
answer is the wrong one.

The roster is a type with no local variant, and every store operation is written against the
shared tier's connection rather than the tier enum, so the per-core map is absent rather than
merely unselected. The key is `System` class (`rule:config/system-means-a-request-may-not-set-it`), because where a fleet's sessions live is a
deployment decision. It reloads: a request reads the block from the snapshot it cloned, so a
changed backend reaches the next request, and no record moves from the old store to the new one,
which signs every user out. An absent block is
not a default backend (`rule:http-server/no-session-block-means-no-store`).
