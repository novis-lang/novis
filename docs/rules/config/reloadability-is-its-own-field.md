Beside its changeability class, every directive carries a second field answering a different
question — not *who may set it* but *what applying a change requires*:

| Field | Meaning |
|---|---|
| `Reload` | a new snapshot is enough |
| `Boot` | applying it would rebind an OS resource or re-create the runtime |

The two are independent. A `System` directive may be `Reload` — a capability grant, `[limits.hard]`,
`[mode] ceiling` — and every `Runtime` and `RuntimeTighten` directive is `Reload` by construction,
since such a directive *is* a value read out of the snapshot. A registry that derived either field
from the other would re-create the conflation this field exists to end, so the census test fails if it
ever does.

**`Boot` is four keys: `[server] listen`, `[server] socket_mode`, `[server] workers` and
`[server] watchdog_margin`.** A port below 1024 needs a privilege the process dropped after it bound,
so `listen` cannot move in general. `socket_mode` is applied when a Unix listener is bound, so it
moves only with `listen`. `workers` sizes the runtime state each core holds. `watchdog_margin` is read
once, by the one watchdog the process builds when it starts. The registry keeps a row per key where a block's keys are
two apply classes: `[server]` is a `Reload` block row with `Boot` rows for its restart keys.

Everything else reloads, including the `[[extension]]` array and its pins, `opcache.validate` and its
rate cap, the per-app blocks, `[[schedule]]`, `[deferred] max_concurrent`, the whole of `[queue]`,
and both observability blocks. A key that names a resource applies by building the new resource from
the published snapshot: work that began before the publish finishes on the old one, which is closed
after the last of it ends. A `[[schedule]]` firing already in flight runs to completion; the new set
arms from the next tick. A queue worker a new `workers` or `connection` stops writes back the job it
holds first, and the workers it starts claim on the new connection, whose storage the reload checks
before it publishes, as the boot does. A new `opcache.file_cache_dir` takes the next compile, and a
directory the ownership check refuses keeps the running one and is named in the log. A reload builds the outbound TLS client `[http.client.tls]` names, reading
its anchor files again, and installs it only when its anchors, version floor or key log differ from
the running client's. The next connection is judged by it, and the pool files every connection under
the client that opened it, so no socket the old anchors accepted serves a later call. A block that
does not build keeps the running client and is named the same way. A request dials the
`[cache.shared]` store its own snapshot names, and the schedule ticker opens its fleet lease again
when that block moves, so the next `fleet` fire takes its key in the new store. A changed `Boot` key
**does not take effect**: the published snapshot carries the running value forward, and the reload
names the key (`rule:config/a-reload-names-what-it-could-not-apply`).
