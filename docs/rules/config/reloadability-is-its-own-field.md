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

**`Boot` is three keys: `[server] listen`, `[server] socket_mode` and `[server] workers`.** A port
below 1024 needs a privilege the process dropped after it bound, so `listen` cannot move in general.
`socket_mode` is applied when a Unix listener is bound, so it moves only with `listen`. `workers`
sizes the runtime state each core holds. The registry keeps a row per key where a block's keys are
two apply classes: `[server]` is a `Reload` block row with `Boot` rows for its restart keys, as
`[opcache]` is with `file_cache_dir`.

Everything else reloads, including the `[[extension]]` array and its pins, `opcache.validate` and its
rate cap, the per-app blocks, `[[schedule]]`, `[deferred] max_concurrent`, the whole of `[queue]`,
and both observability blocks. A key that names a resource applies by building the new resource from
the published snapshot: work that began before the publish finishes on the old one, which is closed
after the last of it ends. A `[[schedule]]` firing already in flight runs to completion; the new set
arms from the next tick. A queue worker a new `workers` or `connection` stops writes back the job it
holds first, and the workers it starts claim on the new connection, whose storage the reload checks
before it publishes, as the boot does. A changed `Boot` key **does not take effect**: the
published snapshot carries the running value forward, and the reload names the key
(`rule:config/a-reload-names-what-it-could-not-apply`).

**What is on disk.** `[server]`'s `dispatch`, `static`, `trusted_proxies`, `health_path`,
`max_in_flight`, the four waits, `drain_timeout`, `[server.connection]`, `root`, `[[server.mount]]`,
`[session]`, `[control] socket`, `[queue]` and `io.temp_root` reload. These rows are still `Boot`,
because each is read once when the server starts: `http.client.tls`, `cache.shared` and
`opcache.file_cache_dir`.
