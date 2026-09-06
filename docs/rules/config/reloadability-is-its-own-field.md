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

`Boot` is the narrow set: `cache.dir`, `[server]`'s listen addresses, the thread-per-core count and
`[control] socket` itself. Everything else reloads, including the `[[extension]]` array and its pins,
`opcache.validate` and its rate cap, the per-app blocks, `[[schedule]]`, `[deferred] max_concurrent`
and both observability blocks. A `[[schedule]]` firing already in flight runs to completion; the new
set arms from the next tick. A changed `Boot` key **does not take effect**: the published snapshot
carries the running value forward, and the reload names the key
(`rule:config/a-reload-names-what-it-could-not-apply`).
