An endpoint an operator wrote into root-owned configuration is authorized **by that writing**, so the
grant over it names the *store* and not a host. `Core\Cache::shared()` asks `cache.shared` at
`Scope::Unscoped`; so does `Core\RateLimit::consume`, which reaches the same store through the same
door and differs only in the sentence it appends to a refusal. Neither asks `net.connect`, and neither
consults `rule:security/net-address-policy`'s denied-range table.

That is `mail.send`'s shape and `db.connect`'s reasoning (`rule:core-classes/db-capabilities`): the
endpoint was written by the authority that grants the capability, so it is pre-approved and there is
nothing for an attacker to influence. A third key of the same kind gets the same treatment rather than
a fourth rule; the cost of the old question was that a loopback Redis needed `net.internal` to except
the very range every ordinary deployment puts its store in.

**Unscoped, because there is nothing to scope on.** A deployment has one shared store; `db.connect` is
scoped by name because there are many `[db.<name>]` blocks. The grant means "this program may reach
the coherent tier", and *where* that tier is stays the operator's answer — so moving it between a
container, a loopback daemon and a socket changes one block and no app's grant list.

A `[cache.shared] url` set with no `cache.shared` grant is reported at boot as a `Warn` naming both
keys, not discovered on a request.

**Not shipped.** `cache.shared` is a directive, not a capability: `Cap` in `nvs-config` has no
`CacheShared` row, and `open_configured` still asks `net.connect` at the URL's host and pins it.
