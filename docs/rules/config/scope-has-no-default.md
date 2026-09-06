Every `[[schedule]]` entry writes `scope`, and there is no default. `"host"` fires the entry on every
host running `nvs serve`, each on its own clock, with no coordination and no lock — right for anything
whose effect is local: warming the per-core cache tier, rotating a local file, sampling host state.
`"fleet"` fires it once per interval across the whole deployment
(`rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`). An entry with neither, or with a third
word, refuses the boot naming both options with one sentence each.

The key is mandatory because both answers are commonly correct and either default silently does the
wrong thing in somebody's production. Defaulting to `"host"` multiplies a fleet's side effects — four
copies of a billing run look exactly like one until the invoices go out. Defaulting to `"fleet"`
silently disables per-host maintenance and makes the shared store a boot dependency for everyone.
One required word per entry removes a class of incident that is otherwise discovered by its
consequences.

**`scope = "fleet"` with no shared store configured refuses to boot**, naming the entry. The store is
`[cache.shared] url` (`rule:core-api/two-cache-tiers`'s coherent tier), and it is the only one. The
alternative — degrading to one run per host with a warning — is the exact failure the key exists to
prevent, and a warning at boot is read once and then never again.
